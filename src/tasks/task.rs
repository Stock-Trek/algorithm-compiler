use crate::{
    aws::Aws,
    dto::sqs_event::{GitProvider, SqsDetail, SqsMessage, SqsRepoDetail},
    dynamodb::DynamoDbDatumRef,
    error::ACResult,
    files::Files,
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
    const DYNAMODB_LOCK_TABLE: &str = "stock-trek-locks";
    const DYNAMODB_LOCK_KEY_ATTRIBUTE: &str = "git-repository";
    const S3_BUCKET_UPLOADS: &str = "stock-trek-uploads";
    const S3_REPOS_PREFIX: &str = "repos";

    async fn handle(&self, aws: &Aws) -> ACResult<()>;
    fn dynamodb_lock_ref(&self, detail: &SqsRepoDetail) -> DynamoDbDatumRef {
        let path = format!("{}/{}", detail.account, detail.repo);
        let key_value = Files::sanitize_path(&path);
        DynamoDbDatumRef {
            table: Self::DYNAMODB_LOCK_TABLE.into(),
            key_name: Self::DYNAMODB_LOCK_KEY_ATTRIBUTE.into(),
            key_value,
        }
    }
    fn repo_prefix(&self, detail: &SqsRepoDetail) -> String {
        let sanitized_account_id = Files::sanitize_path(&detail.account);
        let sanitized_repo_id = Files::sanitize_path(&detail.repo);
        format!(
            "{}/{}/{}",
            Self::S3_REPOS_PREFIX,
            sanitized_account_id,
            sanitized_repo_id
        )
    }
}

#[async_trait]
impl TaskTrait for Task {
    async fn handle(&self, aws: &Aws) -> ACResult<()> {
        match self {
            Self::AddRepos(task) => task.handle(aws).await,
            Self::Commit(task) => task.handle(aws).await,
            Self::RemoveRepos(task) => task.handle(aws).await,
            Self::Rename(task) => task.handle(aws).await,
        }
    }
}

impl From<SqsMessage> for Task {
    fn from(value: SqsMessage) -> Self {
        let SqsMessage { provider, detail } = value;
        match detail {
            SqsDetail::AddRepos { ids } => Self::add_repos_task(provider, ids),
            SqsDetail::Commit {
                ids,
                branch_name,
                commit_hash,
                forced,
            } => Self::commit_task(provider, ids, branch_name, commit_hash, forced),
            SqsDetail::RemoveRepos { ids } => Self::remove_repos_task(provider, ids),
            SqsDetail::Rename { ids, names } => Self::rename_task(provider, ids, names),
        }
    }
}

impl Task {
    fn add_repos_task(provider: GitProvider, ids: Vec<SqsRepoDetail>) -> Task {
        Task::AddRepos(task)
    }
    fn commit_task(
        provider: GitProvider,
        ids: SqsRepoDetail,
        branch_name: String,
        commit_hash: String,
        forced: bool,
    ) -> Task {
        Task::Commit(task)
    }
    fn remove_repos_task(provider: GitProvider, ids: Vec<SqsRepoDetail>) -> Task {
        Task::RemoveRepos(RemoveReposTask { repos: ids })
    }
    fn rename_task(provider: GitProvider, ids: SqsRepoDetail, names: SqsRepoDetail) -> Task {
        Task::Rename(task)
    }
}
