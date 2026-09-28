use crate::{
    aws::Aws,
    dto::sqs_event::{SqsAction, SqsDetail, SqsMessage},
    error::{ACError, ACResult},
    git_remote::GitRemote,
    tasks::{
        add_all_repos::AddAllReposTask, add_ref::AddRefTask, add_repo::AddRepoTask,
        add_repos::AddReposTask, commit::CommitTask, delete_ref::DeleteRefTask,
        remove_all_repos::RemoveAllReposTask, remove_repo::RemoveRepoTask,
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
    AddRepo(AddRepoTask),
    RemoveRepo(RemoveRepoTask),
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
            Self::AddRepo(task) => task.handle(aws, deadline).await,
            Self::RemoveRepo(task) => task.handle(aws, deadline).await,
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
            SqsDetail::AllRepos { action } => match action {
                SqsAction::Add => Self::AddAllRepos(AddAllReposTask::new(source, git_remote)),
                SqsAction::Remove => {
                    Self::RemoveAllRepos(RemoveAllReposTask::new(source, git_remote))
                }
            },
            SqsDetail::Repos { action, repo_ids } => match action {
                SqsAction::Add => Self::AddRepos(AddReposTask::new(source, repo_ids)),
                SqsAction::Remove => Self::RemoveRepos(RemoveReposTask::new(source, repo_ids)),
            },
            SqsDetail::Repo { action, repo_id } => match action {
                SqsAction::Add => Self::AddRepo(AddRepoTask::new(git_remote, repo_id)),
                SqsAction::Remove => Self::RemoveRepo(RemoveRepoTask::new(git_remote, repo_id)),
            },
            SqsDetail::Ref {
                action,
                repo_id,
                ref_name,
                ref_type,
            } => match action {
                SqsAction::Add => {
                    Self::AddRef(AddRefTask::new(git_remote, repo_id, ref_name, ref_type))
                }
                SqsAction::Remove => {
                    Self::DeleteRef(DeleteRefTask::new(git_remote, repo_id, ref_name, ref_type))
                }
            },
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
