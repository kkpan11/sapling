/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;

use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;

use crate::error::Result;
use crate::error::SaplingError;
use crate::types::SaplingStatus;
use crate::utils::get_sapling_executable_path;
use crate::utils::get_sapling_options;
use crate::utils::process_one_status_line;

pub enum SaplingGetDiffDirsResult {
    Normal(Vec<(SaplingStatus, String)>),
    TooManyChanges,
}

pub async fn get_diff_dirs_with_includes(
    first: &str,
    second: Option<&str>,
    limit_results: usize,
    root: &Option<PathBuf>,
    included_roots: Vec<PathBuf>,
) -> Result<SaplingGetDiffDirsResult> {
    get_diff_dirs(
        first,
        second,
        limit_results,
        root,
        &Some(included_roots),
        &None,
    )
    .await
}

// Get directory differences between two revisions. If second is None, then it is the working copy.
// Limit the number of results to limit_results. If the number of results is greater than
// limit_results return TooManyResults. Apply root filters if provided.
pub async fn get_diff_dirs(
    first: &str,
    second: Option<&str>,
    limit_results: usize,
    root: &Option<PathBuf>,
    included_roots: &Option<Vec<PathBuf>>,
    excluded_roots: &Option<Vec<PathBuf>>,
) -> Result<SaplingGetDiffDirsResult> {
    let mut args = vec!["debugdiffdirs", "--rev", first];
    if let Some(second) = second {
        args.push("--rev");
        args.push(second);
    }

    let root_path_arg: String;
    if let Some(root) = root {
        root_path_arg = format!("path:{}", root.display());
        args.push(&root_path_arg);
    };

    let mut output = Command::new(get_sapling_executable_path())
        .envs(get_sapling_options())
        .args(args)
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = output.stdout.take().ok_or_else(|| {
        SaplingError::Other("Failed to read stdout when invoking 'sl debugdiffdirs'.".to_string())
    })?;
    let reader = BufReader::new(stdout);

    let mut status = vec![];
    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await? {
        if let Some(status_line) = process_one_status_line(&line)? {
            if is_path_included(&status_line.1, included_roots, excluded_roots) {
                if status.len() >= limit_results {
                    return Ok(SaplingGetDiffDirsResult::TooManyChanges);
                }
                status.push(status_line);
            }
        }
    }

    Ok(SaplingGetDiffDirsResult::Normal(status))
}

fn is_path_included(
    path: &str,
    included_roots: &Option<Vec<PathBuf>>,
    excluded_roots: &Option<Vec<PathBuf>>,
) -> bool {
    if !included_roots.as_ref().is_none_or(|roots| {
        let path = Path::new(path);
        roots
            .iter()
            .any(|included_root| path.starts_with(included_root))
    }) {
        return false;
    }

    if excluded_roots.as_ref().is_some_and(|roots| {
        let path = Path::new(path);
        roots
            .iter()
            .any(|excluded_root| path.starts_with(excluded_root))
    }) {
        return false;
    }

    true
}
