use crate::{
    aws::Aws,
    dto::sqs_event::{SqsDetail, SqsMessage},
    error::ACResult,
    tasks::{
        add_repos::AddReposTask, commit::CommitTask, remove_repos::RemoveReposTask,
        rename_repo::RenameRepoTask,
    },
};
use async_trait::async_trait;
use std::time::SystemTime;

pub enum Task {
    RenameRepo(RenameRepoTask),
    AddRepos(AddReposTask),
    RemoveRepos(RemoveReposTask),
    Commit(CommitTask),
}

#[async_trait]
pub trait TaskTrait {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()>;
}

#[async_trait]
impl TaskTrait for Task {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        match self {
            Self::RenameRepo(task) => task.handle(aws, deadline).await,
            Self::AddRepos(task) => task.handle(aws, deadline).await,
            Self::RemoveRepos(task) => task.handle(aws, deadline).await,
            Self::Commit(task) => task.handle(aws, deadline).await,
        }
    }
}

impl From<SqsMessage> for Task {
    fn from(value: SqsMessage) -> Self {
        let SqsMessage { provider, detail } = value;
        match detail {
            SqsDetail::AddRepos { ids } => Self::AddRepos(AddReposTask::new(provider, ids)),
            SqsDetail::Commit {
                ids,
                branch_name,
                commit_hash,
                ..
            } => Self::Commit(CommitTask::new(provider, ids, branch_name, commit_hash)),
            SqsDetail::RemoveRepos { ids } => Self::RemoveRepos(RemoveReposTask::new(ids)),
            SqsDetail::RenameRepo { ids, new_names } => {
                Self::RenameRepo(RenameRepoTask::new(provider, ids, new_names))
            }
        }
    }
}
