/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use commit_cloud_types::ClientInfo as CloudClientInfo;
use commit_cloud_types::HistoricalVersion as CloudHistoricalVersion;
use commit_cloud_types::ReferencesData as CloudReferencesData;
use commit_cloud_types::SmartlogData as CloudSmartlogData;
use commit_cloud_types::SmartlogFilter as CloudSmartlogFilter;
use commit_cloud_types::SmartlogFlag;
use commit_cloud_types::SmartlogNode as CloudSmartlogNode;
use commit_cloud_types::UpdateReferencesParams as CloudUpdateReferencesParams;
use commit_cloud_types::WorkspaceData as CloudWorkspaceData;
use commit_cloud_types::WorkspaceRemoteBookmark;
use commit_cloud_types::WorkspaceSharingData as CloudWorkspaceSharingData;
use edenapi_types::cloud::ClientInfo;
use edenapi_types::cloud::ReferencesData;
use edenapi_types::cloud::RemoteBookmark;
use edenapi_types::cloud::SmartlogFilter;
use edenapi_types::GetSmartlogFlag;
use edenapi_types::HgId;
use edenapi_types::HistoricalVersion;
use edenapi_types::SmartlogData;
use edenapi_types::SmartlogNode;
use edenapi_types::UpdateReferencesParams;
use edenapi_types::WorkspaceData;
use edenapi_types::WorkspaceSharingData;
use mercurial_types::HgChangesetId;

pub trait FromCommitCloudType<T> {
    fn from_cc_type(cc: T) -> anyhow::Result<Self>
    where
        Self: std::marker::Sized;
}

pub trait IntoCommitCloudType<T> {
    fn into_cc_type(self) -> anyhow::Result<T>;
}

impl IntoCommitCloudType<CloudUpdateReferencesParams> for UpdateReferencesParams {
    fn into_cc_type(self) -> anyhow::Result<CloudUpdateReferencesParams> {
        Ok(CloudUpdateReferencesParams {
            workspace: self.workspace,
            reponame: self.reponame,
            version: self.version,
            removed_heads: map_hgids(self.removed_heads),
            new_heads: map_hgids(self.new_heads),
            updated_bookmarks: self
                .updated_bookmarks
                .into_iter()
                .map(|(name, node)| (name, node.into()))
                .collect(),
            removed_bookmarks: self.removed_bookmarks,
            updated_remote_bookmarks: self
                .updated_remote_bookmarks
                .map(rbs_into_cc_type)
                .transpose()?,
            removed_remote_bookmarks: self
                .removed_remote_bookmarks
                .map(rbs_into_cc_type)
                .transpose()?,
            new_snapshots: map_hgids(self.new_snapshots),
            removed_snapshots: map_hgids(self.removed_snapshots),
            client_info: self.client_info.map(|ci| ci.into_cc_type()).transpose()?,
        })
    }
}

impl IntoCommitCloudType<CloudClientInfo> for ClientInfo {
    fn into_cc_type(self) -> anyhow::Result<CloudClientInfo> {
        Ok(CloudClientInfo {
            hostname: self.hostname,
            version: self.version,
            reporoot: self.reporoot,
        })
    }
}

impl IntoCommitCloudType<WorkspaceRemoteBookmark> for RemoteBookmark {
    fn into_cc_type(self) -> anyhow::Result<WorkspaceRemoteBookmark> {
        WorkspaceRemoteBookmark::new(self.remote, self.name, self.node.unwrap_or_default().into())
    }
}

impl IntoCommitCloudType<SmartlogFlag> for GetSmartlogFlag {
    fn into_cc_type(self) -> anyhow::Result<SmartlogFlag> {
        Ok(match self {
            GetSmartlogFlag::AddAllBookmarks => SmartlogFlag::AddAllBookmarks,
            GetSmartlogFlag::AddRemoteBookmarks => SmartlogFlag::AddRemoteBookmarks,
            GetSmartlogFlag::SkipPublicCommitsMetadata => SmartlogFlag::SkipPublicCommitsMetadata,
        })
    }
}

impl IntoCommitCloudType<CloudSmartlogFilter> for SmartlogFilter {
    fn into_cc_type(self) -> anyhow::Result<CloudSmartlogFilter> {
        Ok(match self {
            SmartlogFilter::Timestamp(timestamp) => CloudSmartlogFilter::Timestamp(timestamp),
            SmartlogFilter::Version(version) => CloudSmartlogFilter::Version(version),
        })
    }
}

impl FromCommitCloudType<CloudReferencesData> for ReferencesData {
    fn from_cc_type(cc: CloudReferencesData) -> anyhow::Result<Self> {
        Ok(ReferencesData {
            heads: cc.heads.map(map_hgcsids),
            bookmarks: cc.bookmarks.map(|bms| {
                bms.into_iter()
                    .map(|(name, node)| (name, node.into()))
                    .collect()
            }),
            remote_bookmarks: cc.remote_bookmarks.map(rbs_from_cc_type).transpose()?,
            snapshots: cc.snapshots.map(map_hgcsids),
            timestamp: cc.timestamp,
            version: cc.version,
            heads_dates: cc.heads_dates.map(|heads_dates| {
                heads_dates
                    .into_iter()
                    .map(|(hgcsid, date)| (hgcsid.into(), date))
                    .collect()
            }),
        })
    }
}

impl FromCommitCloudType<WorkspaceRemoteBookmark> for RemoteBookmark {
    fn from_cc_type(cc: WorkspaceRemoteBookmark) -> anyhow::Result<RemoteBookmark> {
        Ok(RemoteBookmark {
            name: cc.name().clone(),
            remote: cc.remote().clone(),
            node: Some((*cc.commit()).into()),
        })
    }
}

impl FromCommitCloudType<CloudSmartlogNode> for SmartlogNode {
    fn from_cc_type(cc: CloudSmartlogNode) -> anyhow::Result<Self> {
        Ok(SmartlogNode {
            node: cc.node.into(),
            phase: cc.phase,
            author: cc.author,
            date: cc.date,
            message: cc.message,
            parents: map_hgcsids(cc.parents),
            bookmarks: cc.bookmarks,
            remote_bookmarks: cc.remote_bookmarks.map(rbs_from_cc_type).transpose()?,
        })
    }
}

impl FromCommitCloudType<CloudSmartlogData> for SmartlogData {
    fn from_cc_type(cc: CloudSmartlogData) -> anyhow::Result<Self> {
        Ok(SmartlogData {
            nodes: cc
                .nodes
                .into_iter()
                .map(SmartlogNode::from_cc_type)
                .collect::<anyhow::Result<Vec<SmartlogNode>>>()?,
            version: cc.version,
            timestamp: cc.timestamp,
        })
    }
}

impl FromCommitCloudType<CloudWorkspaceSharingData> for WorkspaceSharingData {
    fn from_cc_type(cc: CloudWorkspaceSharingData) -> anyhow::Result<Self> {
        Ok(WorkspaceSharingData {
            acl_name: cc.acl_name,
            sharing_message: cc.sharing_message,
        })
    }
}

impl FromCommitCloudType<CloudHistoricalVersion> for HistoricalVersion {
    fn from_cc_type(cc: CloudHistoricalVersion) -> anyhow::Result<Self> {
        Ok(HistoricalVersion {
            version_number: cc.version_number,
            timestamp: cc.timestamp,
        })
    }
}

impl FromCommitCloudType<CloudWorkspaceData> for WorkspaceData {
    fn from_cc_type(cc: CloudWorkspaceData) -> anyhow::Result<Self> {
        Ok(WorkspaceData {
            name: cc.name,
            reponame: cc.reponame,
            version: cc.version,
            archived: cc.archived,
            timestamp: cc.timestamp,
        })
    }
}

fn map_hgids(hgids: Vec<HgId>) -> Vec<HgChangesetId> {
    hgids.into_iter().map(|hg| hg.into()).collect()
}

fn map_hgcsids(hgids: Vec<HgChangesetId>) -> Vec<HgId> {
    hgids.into_iter().map(|hg| hg.into()).collect()
}

fn rbs_into_cc_type(rbs: Vec<RemoteBookmark>) -> anyhow::Result<Vec<WorkspaceRemoteBookmark>> {
    rbs.into_iter().map(|rb| rb.into_cc_type()).collect()
}

fn rbs_from_cc_type(fbs: Vec<WorkspaceRemoteBookmark>) -> anyhow::Result<Vec<RemoteBookmark>> {
    fbs.into_iter().map(RemoteBookmark::from_cc_type).collect()
}
