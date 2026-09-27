use crate::{
    aws::Aws,
    dto::sqs_event::GitSource,
    error::ACResult,
    tasks::{add_repos::AddReposTask, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddAllReposTask {
    source: GitSource,
}

impl AddAllReposTask {
    pub fn new(source: GitSource) -> Self {
        Self { source }
    }
}

#[async_trait]
impl TaskTrait for AddAllReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let repo_ids = self.source.account_repo_ids().await?;
        AddReposTask::new(self.source.clone(), repo_ids)
            .handle(aws, deadline)
            .await
    }
}
