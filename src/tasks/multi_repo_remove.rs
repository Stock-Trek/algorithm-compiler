use crate::{
    aws::Aws,
    dto::sqs_event::{GitSource, SqsDetail, SqsMessage, SqsRepoAction},
    error::ACResult,
    tasks::task::TaskTrait,
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct MultiRepoRemoveTask {
    source: GitSource,
    repo_ids: Vec<String>,
}

impl MultiRepoRemoveTask {
    pub fn new(source: GitSource, repo_ids: Vec<String>) -> Self {
        Self { source, repo_ids }
    }
}

#[async_trait]
impl TaskTrait for MultiRepoRemoveTask {
    async fn handle(&self, aws: &Aws, _deadline: SystemTime) -> ACResult<()> {
        for repo_id in &self.repo_ids {
            let message = SqsMessage {
                source: self.source.clone(),
                detail: SqsDetail::Repo {
                    action: SqsRepoAction::Remove,
                    repo_id: repo_id.clone(),
                },
            };
            aws.sqs.send(&message).await?;
        }
        Ok(())
    }
}
