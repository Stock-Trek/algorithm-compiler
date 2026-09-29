use crate::{
    aws::Aws,
    dto::sqs_event::{SqsDetail, SqsMessage, SqsMultiRepoAction, SqsRefAction, SqsRepoAction},
    error::{ACError, ACResult},
    git_remote::GitRemote,
    tasks::{
        all_repos_add::AllReposAddTask, all_repos_remove::AllReposRemoveTask,
        multi_repo_add::MultiRepoAddTask, multi_repo_remove::MultiRepoRemoveTask,
        ref_commit::RefCommitTask, ref_create::RefCreateTask, ref_delete::RefDeleteTask,
        repo_add::RepoAddTask, repo_remove::RepoRemoveTask, repo_rename::RepoRenameTask,
    },
};
use async_trait::async_trait;
use std::time::SystemTime;

pub enum Task {
    AllReposAdd(AllReposAddTask),
    AllReposRemove(AllReposRemoveTask),
    MultiRepoAdd(MultiRepoAddTask),
    MultiRepoRemove(MultiRepoRemoveTask),
    RepoAdd(RepoAddTask),
    RepoRemove(RepoRemoveTask),
    RepoRename(RepoRenameTask),
    RefCreate(RefCreateTask),
    RefDelete(RefDeleteTask),
    RefCommit(RefCommitTask),
}

#[async_trait]
pub trait TaskTrait {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()>;
}

#[async_trait]
impl TaskTrait for Task {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        match self {
            Self::AllReposAdd(task) => task.handle(aws, deadline).await,
            Self::AllReposRemove(task) => task.handle(aws, deadline).await,
            Self::MultiRepoAdd(task) => task.handle(aws, deadline).await,
            Self::MultiRepoRemove(task) => task.handle(aws, deadline).await,
            Self::RepoAdd(task) => task.handle(aws, deadline).await,
            Self::RepoRemove(task) => task.handle(aws, deadline).await,
            Self::RepoRename(task) => task.handle(aws, deadline).await,
            Self::RefCreate(task) => task.handle(aws, deadline).await,
            Self::RefDelete(task) => task.handle(aws, deadline).await,
            Self::RefCommit(task) => task.handle(aws, deadline).await,
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
                SqsMultiRepoAction::Add => {
                    Self::AllReposAdd(AllReposAddTask::new(source, git_remote))
                }
                SqsMultiRepoAction::Remove => {
                    Self::AllReposRemove(AllReposRemoveTask::new(source, git_remote))
                }
            },
            SqsDetail::MultiRepo { action, repo_ids } => match action {
                SqsMultiRepoAction::Add => {
                    Self::MultiRepoAdd(MultiRepoAddTask::new(source, repo_ids))
                }
                SqsMultiRepoAction::Remove => {
                    Self::MultiRepoRemove(MultiRepoRemoveTask::new(source, repo_ids))
                }
            },
            SqsDetail::Repo { action, repo_id } => match action {
                SqsRepoAction::Add => Self::RepoAdd(RepoAddTask::new(git_remote, repo_id)),
                SqsRepoAction::Remove => Self::RepoRemove(RepoRemoveTask::new(git_remote, repo_id)),
                SqsRepoAction::Rename | SqsRepoAction::Transfer => {
                    Self::RepoRename(RepoRenameTask::new(git_remote, repo_id))
                }
            },
            SqsDetail::Ref {
                action,
                repo_id,
                ref_name,
                ref_type,
            } => match action {
                SqsRefAction::Create => {
                    Self::RefCreate(RefCreateTask::new(git_remote, repo_id, ref_name, ref_type))
                }
                SqsRefAction::Delete => {
                    Self::RefDelete(RefDeleteTask::new(git_remote, repo_id, ref_name, ref_type))
                }
            },
            SqsDetail::Commit {
                repo_id,
                branch_name,
                commit_hash,
                ..
            } => Self::RefCommit(RefCommitTask::new(
                git_remote,
                repo_id,
                branch_name,
                commit_hash,
            )),
        })
    }
}
