use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqsEvent {
    #[serde(rename = "Records")]
    pub records: Vec<SqsRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqsRecord {
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SqsMessage {
    pub provider: GitProvider,
    pub detail: SqsDetail,
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, EnumIter, Serialize, Deserialize)]
pub enum GitProvider {
    GitHub,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SqsDetail {
    Rename {
        ids: SqsRepoDetail,
        names: SqsRepoDetail,
    },
    AddRepos {
        ids: Vec<SqsRepoDetail>,
    },
    RemoveRepos {
        ids: Vec<SqsRepoDetail>,
    },
    Commit {
        ids: SqsRepoDetail,
        branch_name: String,
        commit_hash: String,
        forced: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SqsRepoDetail {
    pub account: String,
    pub repo: String,
}
