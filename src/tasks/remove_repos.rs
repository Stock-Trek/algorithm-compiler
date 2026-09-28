use crate::{
    aws::Aws,
    dto::sqs_event::{GitSource, SqsAction, SqsDetail, SqsMessage},
    error::ACResult,
    tasks::task::TaskTrait,
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct RemoveReposTask {
    source: GitSource,
    repo_ids: Vec<String>,
}

impl RemoveReposTask {
    pub fn new(source: GitSource, repo_ids: Vec<String>) -> Self {
        Self { source, repo_ids }
    }
}

#[async_trait]
impl TaskTrait for RemoveReposTask {
    async fn handle(&self, aws: &Aws, _deadline: SystemTime) -> ACResult<()> {
        for repo_id in &self.repo_ids {
            let message = SqsMessage {
                source: self.source.clone(),
                detail: SqsDetail::Repo {
                    action: SqsAction::Remove,
                    repo_id: repo_id.clone(),
                },
            };
            aws.sqs.send(&message).await?;
        }
        Ok(())
    }
}
