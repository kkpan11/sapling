/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::collections::BTreeMap;
use std::sync::atomic;
use std::sync::atomic::AtomicU64;

use super::IdMapWrite;
use crate::Result;
use crate::VerLink;
use crate::errors::NotFoundError;
use crate::id::Group;
use crate::id::Id;
use crate::id::Vertex;
use crate::ops::IdConvert;
use crate::ops::Persist;
use crate::ops::PrefixLookup;

/// Bi-directional mapping between an integer id and a name (`[u8]`).
///
/// Private. Stored in memory.
pub struct MemIdMap {
    core: CoreMemIdMap,
    map_id: String,
    map_version: VerLink,
}

/// Subset of the `MemIdMap` interface that does not have "map_version".
/// or "version" concept.
#[derive(Default, Clone)]
pub(crate) struct CoreMemIdMap {
    id2name: BTreeMap<Id, Vertex>,
    name2id: BTreeMap<Vertex, Id>,
}

impl MemIdMap {
    /// Create an empty [`MemIdMap`].
    pub fn new() -> Self {
        Self {
            core: Default::default(),
            map_id: format!("mem:{}", next_id()),
            map_version: VerLink::new(),
        }
    }
}

impl Clone for MemIdMap {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            map_id: self.map_id.clone(),
            map_version: self.map_version.clone(),
        }
    }
}

impl CoreMemIdMap {
    pub fn lookup_vertex_id(&self, name: &Vertex) -> Option<Id> {
        self.name2id.get(name).copied()
    }

    pub fn lookup_vertex_name(&self, id: Id) -> Option<&Vertex> {
        self.id2name.get(&id)
    }

    pub fn lookup_vertexes_by_hex_prefix(
        &self,
        hex_prefix: &[u8],
        limit: usize,
    ) -> Result<Vec<Vertex>> {
        let start = Vertex::from_hex(hex_prefix)?;
        let mut result = Vec::new();
        for (vertex, id) in self.name2id.range(start..) {
            if result.len() >= limit {
                break;
            }
            if id.is_virtual() && hex_prefix.len() != vertex.as_ref().len() * 2 {
                // Virtual group requires full match.
                continue;
            }
            if !vertex.to_hex().as_bytes().starts_with(hex_prefix) {
                break;
            }
            result.push(vertex.clone());
        }
        Ok(result)
    }

    pub fn lookup_range(&self, low: Id, high: Id) -> impl Iterator<Item = (&Id, &Vertex)> {
        self.id2name.range(low..=high)
    }

    pub fn has_vertex_name(&self, name: &Vertex) -> bool {
        self.name2id.contains_key(name)
    }

    pub fn has_vertex_id(&self, id: Id) -> bool {
        self.id2name.contains_key(&id)
    }

    pub fn insert_vertex_id_name(&mut self, id: Id, vertex_name: Vertex) {
        self.name2id.insert(vertex_name.clone(), id);
        self.id2name.insert(id, vertex_name);
    }

    pub fn remove_range(&mut self, low: Id, high: Id) -> Result<Vec<Vertex>> {
        let to_remove: Vec<(Id, Vertex)> = self
            .id2name
            .range(low..=high)
            .map(|(i, n)| (*i, n.clone()))
            .collect();
        for (id, name) in &to_remove {
            self.id2name.remove(id);
            self.name2id.remove(name);
        }
        Ok(to_remove.into_iter().map(|(_, v)| v).collect())
    }
}

#[async_trait::async_trait]
impl IdConvert for MemIdMap {
    async fn vertex_id(&self, name: Vertex) -> Result<Id> {
        self.core
            .lookup_vertex_id(&name)
            .ok_or_else(|| name.not_found_error())
    }
    async fn vertex_id_with_max_group(
        &self,
        name: &Vertex,
        max_group: Group,
    ) -> Result<Option<Id>> {
        let optional_id = self.core.name2id.get(name).and_then(|id| {
            if id.group() <= max_group {
                Some(*id)
            } else {
                None
            }
        });
        Ok(optional_id)
    }
    async fn vertex_name(&self, id: Id) -> Result<Vertex> {
        self.core
            .lookup_vertex_name(id)
            .cloned()
            .ok_or_else(|| id.not_found_error())
    }
    async fn contains_vertex_name(&self, name: &Vertex) -> Result<bool> {
        Ok(self.core.has_vertex_name(name))
    }

    async fn contains_vertex_id_locally(&self, ids: &[Id]) -> Result<Vec<bool>> {
        Ok(ids
            .iter()
            .copied()
            .map(|id| self.core.has_vertex_id(id))
            .collect())
    }
    async fn contains_vertex_name_locally(&self, names: &[Vertex]) -> Result<Vec<bool>> {
        Ok(names
            .iter()
            .map(|name| self.core.has_vertex_name(name))
            .collect())
    }

    fn map_id(&self) -> &str {
        &self.map_id
    }

    fn map_version(&self) -> &VerLink {
        &self.map_version
    }
}

// TODO: Reconsider re-assign master cases. Currently they are ignored.
#[async_trait::async_trait]
impl IdMapWrite for MemIdMap {
    async fn insert(&mut self, id: Id, name: &[u8]) -> Result<()> {
        let vertex_name = Vertex::copy_from(name);
        self.core.insert_vertex_id_name(id, vertex_name);
        self.map_version.bump();
        Ok(())
    }
    async fn remove_range(&mut self, low: Id, high: Id) -> Result<Vec<Vertex>> {
        if !(low.is_virtual() && high.is_virtual()) {
            self.map_version = VerLink::new();
        }
        self.core.remove_range(low, high)
    }
}

impl Persist for MemIdMap {
    type Lock = ();

    fn lock(&mut self) -> Result<Self::Lock> {
        Ok(())
    }

    fn reload(&mut self, _lock: &Self::Lock) -> Result<()> {
        Ok(())
    }

    fn persist(&mut self, _lock: &Self::Lock) -> Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl PrefixLookup for MemIdMap {
    async fn vertexes_by_hex_prefix(&self, hex_prefix: &[u8], limit: usize) -> Result<Vec<Vertex>> {
        self.core.lookup_vertexes_by_hex_prefix(hex_prefix, limit)
    }
}

fn next_id() -> u64 {
    static ID: AtomicU64 = AtomicU64::new(0);
    ID.fetch_add(1, atomic::Ordering::AcqRel)
}
