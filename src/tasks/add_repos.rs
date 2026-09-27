use crate::{
    aws::Aws,
    dto::sqs_event::GitSource,
    error::ACResult,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddReposTask {
    source: GitSource,
    repo_ids: Vec<String>,
}

impl AddReposTask {
    pub fn new(source: GitSource, repo_ids: Vec<String>) -> Self {
        Self { source, repo_ids }
    }
}

#[async_trait]
impl TaskTrait for AddReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        for repo_id in &self.repo_ids {
            let refs = RepoRefs::new(&aws.config, self.source.clone(), repo_id)?;
            let refs_ref = &refs;
            aws.dynamodb
                .locked(&refs.lock_ref, deadline, move |lock| async move {
                    refs_ref.sync(aws, &lock, deadline).await
                })
                .await?;
        }
        Ok(())
    }
}
