use crate::{
    aws::Aws,
    dto::sqs_event::{SqsDetail, SqsMessage},
    error::ACResult,
    tasks::{
        add_all_repos::AddAllReposTask, add_ref::AddRefTask, add_repos::AddReposTask,
        commit::CommitTask, delete_ref::DeleteRefTask, remove_all_repos::RemoveAllReposTask,
        remove_repos::RemoveReposTask, rename_repo::RenameRepoTask,
    },
};
use async_trait::async_trait;
use std::time::SystemTime;

pub enum Task {
    AddRepos(AddReposTask),
    AddAllRepos(AddAllReposTask),
    RemoveRepos(RemoveReposTask),
    RemoveAllRepos(RemoveAllReposTask),
    RenameRepo(RenameRepoTask),
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
            Self::AddRepos(task) => task.handle(aws, deadline).await,
            Self::AddAllRepos(task) => task.handle(aws, deadline).await,
            Self::RemoveRepos(task) => task.handle(aws, deadline).await,
            Self::RemoveAllRepos(task) => task.handle(aws, deadline).await,
            Self::AddRef(task) => task.handle(aws, deadline).await,
            Self::DeleteRef(task) => task.handle(aws, deadline).await,
            Self::RenameRepo(task) => task.handle(aws, deadline).await,
            Self::Commit(task) => task.handle(aws, deadline).await,
        }
    }
}

impl From<SqsMessage> for Task {
    fn from(value: SqsMessage) -> Self {
        let SqsMessage { detail, .. } = value;
        match detail {
            SqsDetail::AddRepos { ids, .. } => Self::AddRepos(AddReposTask::new(ids)),
            SqsDetail::AddAllRepos {
                account_id,
                account_login,
                ..
            } => Self::AddAllRepos(AddAllReposTask::new(account_id, account_login)),
            SqsDetail::RemoveRepos { ids, .. } => Self::RemoveRepos(RemoveReposTask::new(ids)),
            SqsDetail::RemoveAllRepos { account_id, .. } => {
                Self::RemoveAllRepos(RemoveAllReposTask::new(account_id))
            }
            SqsDetail::AddRef {
                id,
                ref_name,
                ref_type,
                ..
            } => Self::AddRef(AddRefTask::new(id, ref_name, ref_type)),
            SqsDetail::DeleteRef {
                id,
                ref_name,
                ref_type,
                ..
            } => Self::DeleteRef(DeleteRefTask::new(id, ref_name, ref_type)),
            SqsDetail::RenameRepo { id, name, .. } => {
                Self::RenameRepo(RenameRepoTask::new(id, name))
            }
            SqsDetail::Commit {
                id,
                branch_name,
                commit_hash,
                ..
            } => Self::Commit(CommitTask::new(id, branch_name, commit_hash)),
        }
    }
}
