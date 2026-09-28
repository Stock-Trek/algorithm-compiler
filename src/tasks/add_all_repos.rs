use crate::{
    aws::Aws,
    dto::sqs_event::GitSource,
    error::ACResult,
    git_remote::GitRemote,
    tasks::{add_repos::AddReposTask, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddAllReposTask {
    source: GitSource,
    git_remote: GitRemote,
}

impl AddAllReposTask {
    pub fn new(source: GitSource, git_remote: GitRemote) -> Self {
        Self { source, git_remote }
    }
}

#[async_trait]
impl TaskTrait for AddAllReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let repo_ids = self.git_remote.account_repo_ids().await?;
        AddReposTask::new(self.source.clone(), repo_ids)
            .handle(aws, deadline)
            .await
    }
}
