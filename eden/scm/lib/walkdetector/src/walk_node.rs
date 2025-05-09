/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Duration;
use std::time::Instant;

use types::PathComponentBuf;
use types::RepoPath;
use types::RepoPathBuf;

use crate::Walk;
use crate::WalkType;
use crate::interesting_metadata;

/// Tree structure to track active walks. This makes it efficient to find a file's
/// "containing" walk, and to efficiently discover a walk's siblings, cousins, etc. in
/// order to merge walks.
#[derive(Default)]
pub(crate) struct WalkNode {
    // File content walk, if any, rooted at this node.
    pub(crate) file_walk: Option<Walk>,
    // Directory content walk, if any, rooted at this node.
    pub(crate) dir_walk: Option<Walk>,

    pub(crate) last_access: Option<Instant>,
    pub(crate) children: HashMap<PathComponentBuf, WalkNode>,

    // Child directories that have a walked descendant "advanced" past our current
    // walk.depth.
    pub(crate) advanced_file_children: HashSet<PathComponentBuf>,
    pub(crate) advanced_dir_children: HashSet<PathComponentBuf>,

    // Total file count in this directory (if hint available).
    pub(crate) total_files: Option<usize>,
    // Total directory count in this directory (if hint available).
    pub(crate) total_dirs: Option<usize>,
    // File names seen so far (only used before transitioning to walk).
    pub(crate) seen_files: HashSet<PathComponentBuf>,
    // Dir names seen so far (only used before transitioning to walk).
    pub(crate) seen_dirs: HashSet<PathComponentBuf>,
}

impl WalkNode {
    /// Fetch active walk for `walk_root`, if any.
    pub(crate) fn get_walk(&mut self, walk_type: WalkType, walk_root: &RepoPath) -> Option<&Walk> {
        match walk_root.split_first_component() {
            Some((head, tail)) => self
                .children
                .get_mut(head)
                .and_then(|child| child.get_walk(walk_type, tail)),
            None => self.get_dominating_walk(walk_type),
        }
    }

    /// Get existing WalkNode entry for specified root, if any.
    pub(crate) fn get_node(&mut self, walk_root: &RepoPath) -> Option<&mut Self> {
        match walk_root.split_first_component() {
            Some((head, tail)) => self
                .children
                .get_mut(head)
                .and_then(|child| child.get_node(tail)),
            None => Some(self),
        }
    }

    /// Find node with active walk covering directory `dir`, if any.
    pub(crate) fn get_containing_node<'a, 'b>(
        &'a mut self,
        walk_type: WalkType,
        dir: &'b RepoPath,
    ) -> Option<(&'a mut Self, &'b RepoPath)> {
        match dir.split_first_component() {
            Some((head, tail)) => {
                if self.contains(walk_type, dir, 0) {
                    Some((self, dir))
                } else {
                    self.children
                        .get_mut(head)
                        .and_then(|child| child.get_containing_node(walk_type, tail))
                }
            }
            None => {
                if self.get_dominating_walk(walk_type).is_some() {
                    Some((self, dir))
                } else {
                    None
                }
            }
        }
    }

    /// Find node with active walk covering `dir`, or create new node for `dir`. This is a
    /// single step to perform the common get-or-create operation in a single tree
    /// traversal.
    pub(crate) fn get_or_create_owning_node<'a>(
        &'a mut self,
        walk_type: WalkType,
        dir: &'a RepoPath,
    ) -> (&'a mut Self, &'a RepoPath) {
        match dir.split_first_component() {
            Some((head, tail)) => {
                if self.contains(walk_type, dir, 0) {
                    (self, dir)
                } else if self.children.contains_key(head) {
                    self.children
                        .get_mut(head)
                        .unwrap()
                        .get_or_create_owning_node(walk_type, tail)
                } else {
                    self.children
                        .entry(head.to_owned())
                        .or_default()
                        .get_or_create_owning_node(walk_type, tail)
                }
            }
            None => (self, dir),
        }
    }

    /// Find or create node for `dir`.
    pub(crate) fn get_or_create_node<'a>(&'a mut self, dir: &'a RepoPath) -> &'a mut Self {
        match dir.split_first_component() {
            Some((head, tail)) => {
                if self.children.contains_key(head) {
                    self.children
                        .get_mut(head)
                        .unwrap()
                        .get_or_create_node(tail)
                } else {
                    self.children
                        .entry(head.to_owned())
                        .or_default()
                        .get_or_create_node(tail)
                }
            }
            None => self,
        }
    }

    /// Insert a new walk. Any redundant/contained walks will be removed. `walk` will not
    /// be inserted if it is contained by an ancestor walk.
    pub(crate) fn insert_walk(
        &mut self,
        walk_type: WalkType,
        walk_root: &RepoPath,
        mut walk: Walk,
        threshold: usize,
    ) -> &mut Self {
        // If we completely overlap with the walk to be inserted, skip it. This shouldn't
        // happen, but I want to guarantee there are no overlapping walks.
        if self.contains(walk_type, walk_root, walk.depth) {
            return self;
        }

        match walk_root.split_first_component() {
            Some((head, tail)) => {
                if self.children.contains_key(head) {
                    self.children
                        .get_mut(head)
                        .unwrap()
                        .insert_walk(walk_type, tail, walk, threshold)
                } else {
                    self.children
                        .entry(head.to_owned())
                        .or_default()
                        .insert_walk(walk_type, tail, walk, threshold)
                }
            }
            None => {
                self.set_walk_for_type(walk_type, Some(walk));
                self.clear_advanced_children(walk_type);
                self.remove_contained(walk_type, walk.depth, threshold);

                if self.advanced_children_len(walk_type) >= threshold {
                    walk.depth += 1;
                    self.insert_walk(walk_type, walk_root, walk, threshold)
                } else {
                    self
                }
            }
        }
    }

    /// List all active walks.
    pub(crate) fn list_walks(&self, walk_type: WalkType) -> Vec<(RepoPathBuf, Walk)> {
        fn inner(
            node: &WalkNode,
            walk_type: WalkType,
            path: RepoPathBuf,
            list: &mut Vec<(RepoPathBuf, Walk)>,
        ) {
            if let Some(walk) = node.get_walk_for_type(walk_type) {
                list.push((path.clone(), walk.clone()));
            }

            for (name, child) in node.children.iter() {
                inner(child, walk_type, path.join(name.as_path_component()), list);
            }
        }

        let mut list = Vec::new();
        inner(self, walk_type, RepoPathBuf::new(), &mut list);
        list
    }

    pub(crate) fn child_walks(
        &self,
        walk_type: WalkType,
    ) -> impl Iterator<Item = (&PathComponentBuf, &Walk)> {
        self.children
            .iter()
            .filter_map(move |(name, node)| node.get_walk_for_type(walk_type).map(|w| (name, w)))
    }

    /// Get most "powerful" walk that covers `walk_type`. Basically, a file walk covers a
    /// directory walk, so if walk_type=Directory, we return `self.file_walk ||
    /// self.dir_walk`.
    pub(crate) fn get_dominating_walk(&self, walk_type: WalkType) -> Option<&Walk> {
        match walk_type {
            WalkType::File => self.file_walk.as_ref(),
            WalkType::Directory => self.file_walk.as_ref().or(self.dir_walk.as_ref()),
        }
    }

    pub(crate) fn get_walk_for_type(&self, walk_type: WalkType) -> Option<&Walk> {
        match walk_type {
            WalkType::File => self.file_walk.as_ref(),
            WalkType::Directory => self.dir_walk.as_ref(),
        }
    }

    fn set_walk_for_type(&mut self, walk_type: WalkType, walk: Option<Walk>) {
        match walk_type {
            WalkType::File => self.file_walk = walk,
            WalkType::Directory => self.dir_walk = walk,
        }

        // File walk implies directory walk, so clear out contained directory walk.
        match (walk_type, walk, self.dir_walk) {
            (WalkType::File, Some(Walk { depth }), Some(Walk { depth: dir_depth }))
                if depth >= dir_depth =>
            {
                self.dir_walk = None;
            }
            _ => {}
        }
    }

    pub(crate) fn insert_advanced_child(
        &mut self,
        walk_type: WalkType,
        name: PathComponentBuf,
    ) -> usize {
        match walk_type {
            WalkType::File => {
                self.advanced_file_children.insert(name);
                self.advanced_file_children.len()
            }
            WalkType::Directory => {
                self.advanced_dir_children.insert(name);
                self.advanced_dir_children.len()
            }
        }
    }

    fn advanced_children_len(&self, walk_type: WalkType) -> usize {
        match walk_type {
            WalkType::File => self.advanced_file_children.len(),
            WalkType::Directory => self.advanced_dir_children.len(),
        }
    }

    fn clear_advanced_children(&mut self, walk_type: WalkType) {
        match walk_type {
            WalkType::File => self.advanced_file_children.clear(),
            WalkType::Directory => self.advanced_dir_children.clear(),
        }
    }

    /// Recursively remove all walks contained within a walk of depth `depth`.
    fn remove_contained(&mut self, walk_type: WalkType, depth: usize, threshold: usize) {
        // Returns whether a walk exists at depth+1.
        fn inner(
            node: &mut WalkNode,
            walk_type: WalkType,
            depth: usize,
            top: bool,
            threshold: usize,
        ) -> bool {
            let mut any_child_advanced = false;
            let mut new_advanced_children = Vec::new();
            node.children.retain(|name, child| {
                let mut child_advanced = false;

                if child
                    .get_walk_for_type(walk_type)
                    .is_some_and(|w| w.depth >= depth)
                {
                    child_advanced = true;
                } else {
                    child.set_walk_for_type(walk_type, None);
                }

                if depth > 0 {
                    if inner(child, walk_type, depth - 1, false, threshold) {
                        child_advanced = true;
                    }
                }

                if top && child_advanced {
                    // Record if this top-level child has advanced children, meaning a
                    // descendant walk that has pushed to depth+1.
                    tracing::trace!(%name, "inserting advanced child during removal");
                    new_advanced_children.push(name.to_owned());
                }

                any_child_advanced = any_child_advanced || child_advanced;

                child.file_walk.is_some() || child.dir_walk.is_some()
                    || !child.children.is_empty()
                    // Keep node around if it has total file/dir hints that are likely to be useful.
                    || interesting_metadata(threshold, child.total_files, child.total_dirs)
            });

            for advanced in new_advanced_children {
                node.insert_advanced_child(walk_type, advanced);
            }

            any_child_advanced
        }

        inner(self, walk_type, depth, true, threshold);
    }

    /// Reports whether self has a walk and the walk fully contains a descendant walk
    /// rooted at `path` of depth `depth`.
    fn contains(&self, walk_type: WalkType, path: &RepoPath, depth: usize) -> bool {
        self.get_dominating_walk(walk_type)
            .is_some_and(|w| w.depth >= (path.components().count() + depth))
    }

    /// Return whether this Dir should be considered "walked".
    pub(crate) fn is_walked(&self, walk_type: WalkType, dir_walk_threshold: usize) -> bool {
        match walk_type {
            WalkType::File => {
                self.seen_files.len() >= dir_walk_threshold
                    || self
                        .total_files
                        .is_some_and(|total| total < dir_walk_threshold)
            }
            WalkType::Directory => {
                self.seen_dirs.len() >= dir_walk_threshold
                    || self
                        .total_dirs
                        .is_some_and(|total| total < dir_walk_threshold)
            }
        }
    }

    pub(crate) fn iter(&self, mut cb: impl FnMut(&WalkNode, usize) -> bool) {
        fn inner(node: &WalkNode, cb: &mut impl FnMut(&WalkNode, usize) -> bool, depth: usize) {
            if !cb(node, depth) {
                return;
            }

            for child in node.children.values() {
                inner(child, cb, depth + 1);
            }
        }

        inner(self, &mut cb, 0);
    }

    /// Delete nodes not accessed within timeout.
    /// Returns (num_deleted, num_remaining).
    #[tracing::instrument(level = "debug", skip(self))]
    pub(crate) fn gc(&mut self, timeout: Duration, now: Instant) -> (usize, usize) {
        // Return (num_deleted, num_remaining, keep_me)
        fn inner(
            name: &str,
            node: &mut WalkNode,
            timeout: Duration,
            now: Instant,
        ) -> (usize, usize, bool) {
            let mut deleted = 0;
            let mut retained = 0;

            node.children.retain(|name, child| {
                let (d, r, keep) = inner(name.as_str(), child, timeout, now);

                deleted += d;
                retained += r;

                if !keep {
                    tracing::debug!(%name, "GCing node");
                }

                keep
            });

            let expired = node
                .last_access
                .is_none_or(|accessed| now - accessed >= timeout);

            let keep_me = !expired || !node.children.is_empty();

            if expired && keep_me {
                tracing::debug!(%name, "GCing node with children");
                node.clear_except_children();
            }

            (deleted, retained, keep_me)
        }

        let (deleted, remaining, keep_me) = inner("", self, timeout, now);
        if !keep_me {
            // At top level we have no parent to remove us, so just unset our fields.
            tracing::debug!("GCing root node");
            self.clear_except_children();
        }

        (deleted, remaining)
    }

    // Clear all fields except children.
    fn clear_except_children(&mut self) {
        self.file_walk.take();
        self.dir_walk.take();
        self.last_access.take();
        self.advanced_file_children.clear();
        self.advanced_dir_children.clear();
        self.total_files.take();
        self.total_dirs.take();
        self.seen_files.clear();
    }
}
