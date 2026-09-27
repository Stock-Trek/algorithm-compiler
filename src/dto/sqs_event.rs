use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqsEvent {
    #[serde(rename = "Records")]
    pub records: Vec<SqsRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqsRecord {
    pub message_id: String,
    pub body: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqsEventResponse {
    pub batch_item_failures: Vec<SqsBatchItemFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqsBatchItemFailure {
    pub item_identifier: String,
}

impl SqsEventResponse {
    pub fn add_failure(&mut self, message_id: impl Into<String>) {
        self.batch_item_failures.push(SqsBatchItemFailure {
            item_identifier: message_id.into(),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SqsMessage {
    pub provider: GitProvider,
    pub detail: SqsDetail,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Serialize, Deserialize)]
pub enum GitProvider {
    GitHub,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SqsDetail {
    AddRepos {
        installation_id: i64,
        ids: Vec<SqsRepoId>,
    },
    RemoveRepos {
        installation_id: i64,
        ids: Vec<SqsRepoId>,
    },
    AddAllRepos {
        installation_id: i64,
        account_id: String,
        account_login: String,
    },
    RemoveAllRepos {
        installation_id: i64,
        account_id: String,
        account_login: String,
    },
    RenameRepo {
        installation_id: i64,
        id: SqsRepoId,
        name: SqsRepoName,
    },
    AddRef {
        installation_id: i64,
        id: SqsRepoId,
        ref_name: String,
        ref_type: SqsRefType,
    },
    DeleteRef {
        installation_id: i64,
        id: SqsRepoId,
        ref_name: String,
        ref_type: SqsRefType,
    },
    Commit {
        installation_id: i64,
        id: SqsRepoId,
        branch_name: String,
        commit_hash: String,
        forced: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SqsRepoId {
    pub account_id: String,
    pub repo_id: String,
    pub clone_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SqsRepoName {
    pub account: String,
    pub repo: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqsRefType {
    Branch,
    Tag,
}
