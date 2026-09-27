use serde::{Deserialize, Serialize};
use strum::Display;

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
    pub source: GitSource,
    pub detail: SqsDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SqsDetail {
    AddAllRepos,
    RemoveAllRepos,
    AddRepos {
        repo_ids: Vec<String>,
    },
    RemoveRepos {
        repo_ids: Vec<String>,
    },
    Rename {
        repo_id: String,
    },
    AddRef {
        repo_id: String,
        ref_name: String,
        ref_type: SqsRefType,
    },
    DeleteRef {
        repo_id: String,
        ref_name: String,
        ref_type: SqsRefType,
    },
    Commit {
        repo_id: String,
        branch_name: String,
        commit_hash: String,
        forced: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqsRefType {
    Branch,
    Tag,
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitSource {
    GitHub {
        delivery_id: String,
        installation_id: u64,
    },
}
