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
        for id in &self.repo_ids {
            let clone_url = self.source.repo(aws, id.repo_number()?).await?.clone_url;
            let refs = RepoRefs::new(&aws.config, id)?;
            let refs_ref = &refs;
            aws.dynamodb
                .locked(&refs.lock_ref, deadline, move |lock| async move {
                    refs_ref.sync(&clone_url, None, aws, &lock, deadline).await
                })
                .await?;
        }
        Ok(())
    }
}
