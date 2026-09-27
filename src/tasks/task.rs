use crate::{
    aws::Aws,
    dto::sqs_event::{SqsDetail, SqsMessage},
    error::ACResult,
    tasks::{
        add_all_repos::AddAllReposTask, add_ref::AddRefTask, add_repos::AddReposTask,
        commit::CommitTask, delete_ref::DeleteRefTask, remove_all_repos::RemoveAllReposTask,
        remove_repos::RemoveReposTask, rename::RenameTask,
    },
};
use async_trait::async_trait;
use std::time::SystemTime;

pub enum Task {
    AddAllRepos(AddAllReposTask),
    RemoveAllRepos(RemoveAllReposTask),
    AddRepos(AddReposTask),
    RemoveRepos(RemoveReposTask),
    Rename(RenameTask),
    AddRef(AddRefTask),
    DeleteRef(DeleteRefTask),
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
            Self::AddAllRepos(task) => task.handle(aws, deadline).await,
            Self::RemoveAllRepos(task) => task.handle(aws, deadline).await,
            Self::AddRepos(task) => task.handle(aws, deadline).await,
            Self::RemoveRepos(task) => task.handle(aws, deadline).await,
            Self::AddRef(task) => task.handle(aws, deadline).await,
            Self::DeleteRef(task) => task.handle(aws, deadline).await,
            Self::Rename(task) => task.handle(aws, deadline).await,
            Self::Commit(task) => task.handle(aws, deadline).await,
        }
    }
}

impl From<SqsMessage> for Task {
    fn from(value: SqsMessage) -> Self {
        let SqsMessage { source, detail } = value;
        match detail {
            SqsDetail::AddAllRepos => Self::AddAllRepos(AddAllReposTask::new(source)),
            SqsDetail::RemoveAllRepos => Self::RemoveAllRepos(RemoveAllReposTask::new(source)),
            SqsDetail::AddRepos { repo_ids } => Self::AddRepos(AddReposTask::new(source, repo_ids)),
            SqsDetail::RemoveRepos { repo_ids } => {
                Self::RemoveRepos(RemoveReposTask::new(source, repo_ids))
            }
            SqsDetail::AddRef {
                repo_id,
                ref_name,
                ref_type,
            } => Self::AddRef(AddRefTask::new(source, repo_id, ref_name, ref_type)),
            SqsDetail::DeleteRef {
                repo_id,
                ref_name,
                ref_type,
            } => Self::DeleteRef(DeleteRefTask::new(source, repo_id, ref_name, ref_type)),
            SqsDetail::Rename { repo_id } => Self::Rename(RenameTask::new(source, repo_id)),
            SqsDetail::Commit {
                repo_id,
                branch_name,
                commit_hash,
                ..
            } => Self::Commit(CommitTask::new(source, repo_id, branch_name, commit_hash)),
        }
    }
}
