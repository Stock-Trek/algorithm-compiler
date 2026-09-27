use crate::{
    aws::Aws,
    dto::sqs_event::{SqsDetail, SqsMessage},
    error::{ACError, ACResult},
    git_remote::GitRemote,
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

impl TryFrom<SqsMessage> for Task {
    type Error = ACError;

    fn try_from(value: SqsMessage) -> Result<Self, Self::Error> {
        let SqsMessage { source, detail } = value;
        let git_remote: GitRemote = (&source).try_into()?;
        Ok(match detail {
            SqsDetail::AddAllRepos => Self::AddAllRepos(AddAllReposTask::new(git_remote)),
            SqsDetail::RemoveAllRepos => Self::RemoveAllRepos(RemoveAllReposTask::new(git_remote)),
            SqsDetail::AddRepos { repo_ids } => {
                Self::AddRepos(AddReposTask::new(git_remote, repo_ids))
            }
            SqsDetail::RemoveRepos { repo_ids } => {
                Self::RemoveRepos(RemoveReposTask::new(git_remote, repo_ids))
            }
            SqsDetail::AddRef {
                repo_id,
                ref_name,
                ref_type,
            } => Self::AddRef(AddRefTask::new(git_remote, repo_id, ref_name, ref_type)),
            SqsDetail::DeleteRef {
                repo_id,
                ref_name,
                ref_type,
            } => Self::DeleteRef(DeleteRefTask::new(git_remote, repo_id, ref_name, ref_type)),
            SqsDetail::Rename { repo_id } => Self::Rename(RenameTask::new(git_remote, repo_id)),
            SqsDetail::Commit {
                repo_id,
                branch_name,
                commit_hash,
                ..
            } => Self::Commit(CommitTask::new(
                git_remote,
                repo_id,
                branch_name,
                commit_hash,
            )),
        })
    }
}
