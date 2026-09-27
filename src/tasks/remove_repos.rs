use crate::{
    aws::Aws,
    dto::sqs_event::{GitProvider, SqsRepoId},
    error::ACResult,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct RemoveReposTask {
    provider: GitProvider,
    ids: Vec<SqsRepoId>,
}

impl RemoveReposTask {
    pub fn new(provider: GitProvider, ids: Vec<SqsRepoId>) -> Self {
        Self { provider, ids }
    }
}

#[async_trait]
impl TaskTrait for RemoveReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        for id in &self.ids {
            let refs = RepoRefs::new(&aws.config, id)?;
            let prefix = RepoRefs::prefix(id)?;
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
