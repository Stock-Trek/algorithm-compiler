use crate::{
    aws::Aws,
    dto::sqs_event::{GitProvider, SqsRepoId},
    error::ACResult,
    tasks::{add_repos::AddReposTask, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddAllReposTask {
    provider: GitProvider,
    account_id: String,
}

impl AddAllReposTask {
    pub fn new(provider: GitProvider, account_id: String) -> Self {
        Self {
            provider,
            account_id,
        }
    }
}

#[async_trait]
impl TaskTrait for AddAllReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let ids = self
            .provider
            .repos(aws)
            .await?
            .into_iter()
            .map(|repo| SqsRepoId {
                account_id: self.account_id.clone(),
                repo_id: repo.id.to_string(),
            })
            .collect();
        AddReposTask::new(self.provider.clone(), ids)
            .handle(aws, deadline)
            .await
    }
}
