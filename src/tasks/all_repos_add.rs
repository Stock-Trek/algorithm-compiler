use crate::{
    aws::Aws,
    dto::sqs_event::GitSource,
    error::ACResult,
    git_remote::GitRemote,
    tasks::{multi_repo_add::MultiRepoAddTask, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AllReposAddTask {
    source: GitSource,
    git_remote: GitRemote,
}

impl AllReposAddTask {
    pub fn new(source: GitSource, git_remote: GitRemote) -> Self {
        Self { source, git_remote }
    }
}

#[async_trait]
impl TaskTrait for AllReposAddTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let repo_ids = self.git_remote.account_repo_ids().await?;
        MultiRepoAddTask::new(self.source.clone(), repo_ids)
            .handle(aws, deadline)
            .await
    }
}
