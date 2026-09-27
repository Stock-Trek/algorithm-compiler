use crate::{
    aws::Aws,
    dto::sqs_event::GitSource,
    error::ACResult,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
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
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        for repo_id in &self.repo_ids {
            let refs = RepoRefs::new(&aws.config, repo_id)?;
            let prefix = RepoRefs::prefix(repo_id)?;
            let refs_ref = &refs;
            let prefix_ref = &prefix;
            aws.dynamodb
                .locked(&refs.lock_ref, deadline, move |lock| async move {
                    aws.fenced_s3(&refs_ref.lock_ref, &lock)
                        .delete_objects_with_prefix(&refs_ref.repo_ref.bucket, prefix_ref)
                        .await
                })
                .await?;
        }
        Ok(())
    }
}
