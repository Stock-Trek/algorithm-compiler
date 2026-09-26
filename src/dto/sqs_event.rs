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
#[serde(rename_all = "camelCase")]
pub struct SqsMessage {
    pub provider: GitProvider,
    pub detail: SqsDetail,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GitProvider {
    GitHub,
}

impl GitProvider {
    pub fn clone_url(&self, account: &str, repo: &str) -> String {
        match self {
            Self::GitHub => format!("https://github.com/{account}/{repo}.git"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SqsDetail {
    RenameRepo {
        ids: SqsRepoDetail,
        new_names: SqsRepoDetail,
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
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqsRepoDetail {
    pub account: String,
    pub repo: String,
}
