use crate::{
    aws::Aws,
    dto::sqs_event::SqsRepoDetail,
    error::ACResult,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct RemoveReposTask {
    repos: Vec<SqsRepoDetail>,
}

impl RemoveReposTask {
    pub fn new(repos: Vec<SqsRepoDetail>) -> Self {
        Self { repos }
    }
}

#[async_trait]
impl TaskTrait for RemoveReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        for detail in &self.repos {
            let refs = RepoRefs::new(&aws.config, detail)?;
            let prefix = RepoRefs::prefix(detail)?;
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
