use crate::{
    aws::Aws,
    dto::sqs_event::SqsRepoDetail,
    error::ACResult,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;

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
    async fn handle(&self, aws: &Aws) -> ACResult<()> {
        for detail in &self.repos {
            let refs = RepoRefs::new(detail);
            let prefix = RepoRefs::prefix(detail);
            aws.dynamodb
                .locked(&refs.lock_ref, async {
                    aws.s3
                        .delete_objects_with_prefix(&refs.repo_ref.bucket, &prefix)
                        .await
                })
                .await?;
        }
        Ok(())
    }
}
