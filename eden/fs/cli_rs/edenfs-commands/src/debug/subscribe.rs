/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! edenfsctl debug subscribe

#[cfg(unix)]
use std::ffi::OsStr;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use edenfs_client::types::JournalPosition;
use edenfs_client::utils::get_mount_point;
use edenfs_client::EdenFsInstance;
use hg_util::path::expand_path;
use serde::Serialize;
use thrift_types::edenfs as edenfs_thrift;
use tokio::io::AsyncWriteExt;
use tokio::sync::Notify;

use crate::util::jsonrpc::ResponseBuilder;
use crate::ExitCode;

// Defines a few helper functions to make the debug format easier to read.
mod fmt {
    use std::fmt;
    use std::fmt::Debug;

    use thrift_types::edenfs as edenfs_thrift;

    /// Courtesy of https://users.rust-lang.org/t/reusing-an-fmt-formatter/8531/4
    ///
    /// This allows us to provide customized format implementation to avoid
    /// using the default one.
    pub struct Fmt<F>(pub F)
    where
        F: Fn(&mut fmt::Formatter) -> fmt::Result;

    impl<F> fmt::Debug for Fmt<F>
    where
        F: Fn(&mut fmt::Formatter) -> fmt::Result,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            (self.0)(f)
        }
    }

    fn debug_hash(hash: &edenfs_thrift::ThriftRootId) -> impl Debug + '_ {
        Fmt(move |f| write!(f, "{}", hex::encode(hash)))
    }

    fn debug_position(position: &edenfs_thrift::JournalPosition) -> impl Debug + '_ {
        Fmt(|f| {
            f.debug_struct("JournalPosition")
                .field("mountGeneration", &position.mountGeneration)
                .field("sequenceNumber", &position.sequenceNumber)
                .field("snapshotHash", &debug_hash(&position.snapshotHash))
                .finish()
        })
    }

    fn debug_path(path: &edenfs_thrift::PathString) -> impl Debug + '_ {
        Fmt(|f| write!(f, "{}", String::from_utf8_lossy(path)))
    }

    pub fn debug_file_delta(delta: &edenfs_thrift::FileDelta) -> impl Debug + '_ {
        Fmt(|f| {
            f.debug_struct("FileDelta")
                .field("fromPosition", &debug_position(&delta.fromPosition))
                .field("toPosition", &debug_position(&delta.toPosition))
                .field(
                    "changedPaths",
                    &Fmt(|f| {
                        f.debug_list()
                            .entries(delta.changedPaths.iter().map(debug_path))
                            .finish()
                    }),
                )
                .field(
                    "createdPaths",
                    &Fmt(|f| {
                        f.debug_list()
                            .entries(delta.createdPaths.iter().map(debug_path))
                            .finish()
                    }),
                )
                .field(
                    "uncleanPaths",
                    &Fmt(|f| {
                        f.debug_list()
                            .entries(delta.uncleanPaths.iter().map(debug_path))
                            .finish()
                    }),
                )
                .field(
                    "snapshotTransitions",
                    &Fmt(|f| {
                        f.debug_list()
                            .entries(delta.uncleanPaths.iter().map(debug_hash))
                            .finish()
                    }),
                )
                .finish()
        })
    }
}

#[derive(Debug, Serialize)]
struct SubscribeResponse {
    mount_generation: i64,
    sequence_number: u64,
    snapshot_hash: String,
}

impl From<JournalPosition> for SubscribeResponse {
    fn from(from: JournalPosition) -> Self {
        Self {
            mount_generation: from.mount_generation,
            sequence_number: from.sequence_number,
            snapshot_hash: hex::encode(from.snapshot_hash),
        }
    }
}

#[derive(Parser, Debug)]
#[clap(about = "Subscribes to journal changes. Responses are in JSON format")]
pub struct SubscribeCmd {
    #[clap(parse(from_str = expand_path))]
    /// Path to the mount point
    mount_point: Option<PathBuf>,

    #[clap(short, long, default_value = "500")]
    /// [Unit: ms] number of milliseconds to wait between events
    throttle: u64,

    #[clap(short, long, default_value = "15")]
    /// [Unit: seconds] number of seconds to trigger an arbitrary check of
    /// current journal position in case of event missing.
    guard: u64,
}

fn have_non_hg_changes(changes: &[edenfs_thrift::PathString]) -> bool {
    changes.iter().any(|f| !f.starts_with(b".hg"))
}

fn decide_should_notify(changes: edenfs_thrift::FileDelta) -> bool {
    // If the commit hash has changed, report them
    if changes.fromPosition.snapshotHash != changes.toPosition.snapshotHash {
        return true;
    }
    // If we see any non-Mercurial changes, report them
    if have_non_hg_changes(&changes.createdPaths) {
        return true;
    }
    if have_non_hg_changes(&changes.removedPaths) {
        return true;
    }
    if have_non_hg_changes(&changes.uncleanPaths) {
        return true;
    }
    if have_non_hg_changes(&changes.changedPaths) {
        return true;
    }
    // Otherwise, do not notify
    false
}

impl SubscribeCmd {
    async fn _make_notify_event(
        mount_point: &Vec<u8>,
        mount_point_path: &Option<PathBuf>,
        last_position: &mut Option<edenfs_client::types::JournalPosition>,
    ) -> Option<ResponseBuilder> {
        let instance = EdenFsInstance::global();

        let journal = match instance.get_journal_position(mount_point_path, None).await {
            Ok(journal) => journal,
            Err(e) => {
                return Some(ResponseBuilder::error(&format!(
                    "error while getting current journal position: {e:?}",
                )));
            }
        };

        let client = match instance.connect(None).await {
            Ok(client) => client,
            Err(e) => {
                return Some(ResponseBuilder::error(&format!(
                    "error while establishing connection to EdenFS server {e:?}"
                )));
            }
        };

        let should_notify = if let Some(last_position) = last_position.replace(journal.clone()) {
            if last_position.sequence_number == journal.sequence_number {
                tracing::trace!(
                    ?journal,
                    ?last_position,
                    "skipping this event since sequence number matches"
                );
                return None;
            }

            let changes = client
                .getFilesChangedSince(mount_point, &last_position.into())
                .await;

            match changes {
                Ok(changes) => {
                    tracing::debug!(delta = ?fmt::debug_file_delta(&changes));
                    decide_should_notify(changes)
                }
                Err(e) => {
                    return Some(ResponseBuilder::error(&format!(
                        "error while querying changed files {:?}",
                        e
                    )));
                }
            }
        } else {
            false
        };

        if should_notify {
            let result = match serde_json::to_value(SubscribeResponse::from(journal)) {
                Err(e) => ResponseBuilder::error(&format!(
                    "error while serializing subscription response: {e:?}",
                )),
                Ok(serialized) => ResponseBuilder::result(serialized),
            };
            Some(result)
        } else {
            None
        }
    }
}

#[async_trait]
impl crate::Subcommand for SubscribeCmd {
    #[cfg(not(fbcode_build))]
    async fn run(&self) -> Result<ExitCode> {
        eprintln!("not supported in non-fbcode build");
        Ok(1)
    }

    #[cfg(fbcode_build)]
    async fn run(&self) -> Result<ExitCode> {
        let instance = EdenFsInstance::global();

        let mount_point_path = get_mount_point(&self.mount_point)?;
        #[cfg(unix)]
        let mount_point = <Path as AsRef<OsStr>>::as_ref(&mount_point_path)
            .to_os_string()
            .into_vec();
        // SAFETY: paths on Windows are Unicode
        #[cfg(windows)]
        let mount_point = mount_point_path.to_string_lossy().into_owned().into_bytes();

        let notify = Arc::new(Notify::new());

        tokio::task::spawn({
            let notify = notify.clone();
            let mount_point = mount_point.clone();
            let mount_point_path = mount_point_path.to_path_buf();

            async move {
                let mut stdout = tokio::io::stdout();

                {
                    let response = ResponseBuilder::result(serde_json::json!({
                        "message": format!("subscribed to {}", mount_point_path.display())
                    }))
                    .build();
                    let mut bytes = serde_json::to_vec(&response).unwrap();
                    bytes.push(b'\n');
                    stdout.write_all(&bytes).await.ok();
                }

                let mount_point_path_opt = Some(mount_point_path);

                let mut last_position = match instance
                    .get_journal_position(&mount_point_path_opt, None)
                    .await
                {
                    Ok(journal) => Some(journal),
                    Err(_) => None,
                };

                loop {
                    notify.notified().await;
                    let response = match Self::_make_notify_event(
                        &mount_point,
                        &mount_point_path_opt,
                        &mut last_position,
                    )
                    .await
                    {
                        None => continue,
                        Some(response) => response.build(),
                    };

                    match serde_json::to_vec(&response) {
                        Ok(mut bytes) => {
                            bytes.push(b'\n');
                            stdout.write_all(&bytes).await.ok();
                        }
                        Err(e) => {
                            tracing::error!(?e, ?response, "unable to seralize response to JSON");
                        }
                    }
                }
            }
        });

        instance
            .subscribe(&mount_point, self.throttle, self.guard, notify)
            .await?;

        Ok(0)
    }
}
