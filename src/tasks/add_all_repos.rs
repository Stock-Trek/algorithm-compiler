use crate::{
    aws::Aws,
    dto::sqs_event::SqsRepoId,
    error::ACResult,
    tasks::{add_repos::AddReposTask, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddAllReposTask {
    account_id: String,
    installation_id: i64,
}

impl AddAllReposTask {
    pub fn new(account_id: String, installation_id: i64) -> Self {
        Self {
            account_id,
            installation_id,
        }
    }
}

#[async_trait]
impl TaskTrait for AddAllReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let ids = aws
            .github
            .repos(self.installation_id)
            .await?
            .into_iter()
            .map(|repo| SqsRepoId {
                account_id: self.account_id.clone(),
                repo_id: repo.id.to_string(),
                clone_url: repo.clone_url,
            })
            .collect();
        AddReposTask::new(ids).handle(aws, deadline).await
    }
}
