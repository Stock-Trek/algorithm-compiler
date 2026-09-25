use crate::{
    aws::Aws,
    dto::sqs_event::{SqsDetail, SqsMessage},
    error::ACResult,
    tasks::{
        add_repos::AddReposTask, commit::CommitTask, remove_repos::RemoveReposTask,
        rename::RenameTask,
    },
};
use async_trait::async_trait;

pub enum Task {
    Rename(RenameTask),
    AddRepos(AddReposTask),
    RemoveRepos(RemoveReposTask),
    Commit(CommitTask),
}

#[async_trait]
pub trait TaskTrait {
    async fn handle(&self, aws: &Aws) -> ACResult<()>;
}

#[async_trait]
impl TaskTrait for Task {
    async fn handle(&self, aws: &Aws) -> ACResult<()> {
        match self {
            Self::Rename(task) => task.handle(aws).await,
            Self::AddRepos(task) => task.handle(aws).await,
            Self::RemoveRepos(task) => task.handle(aws).await,
            Self::Commit(task) => task.handle(aws).await,
        }
    }
}

impl From<SqsMessage> for Task {
    fn from(value: SqsMessage) -> Self {
        let SqsMessage { detail, .. } = value;
        match detail {
            SqsDetail::AddRepos { ids } => Self::AddRepos(AddReposTask::new(ids)),
            SqsDetail::Commit {
                ids,
                branch_name,
                commit_hash,
                ..
            } => Self::Commit(CommitTask::new(ids, branch_name, commit_hash)),
            SqsDetail::RemoveRepos { ids } => Self::RemoveRepos(RemoveReposTask::new(ids)),
            SqsDetail::Rename { ids, names } => Self::Rename(RenameTask::new(ids, names)),
        }
    }
}
