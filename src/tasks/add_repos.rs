use crate::{
    aws::Aws,
    dto::sqs_event::{GitProvider, SqsRepoId},
    error::ACResult,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddReposTask {
    provider: GitProvider,
    ids: Vec<SqsRepoId>,
}

impl AddReposTask {
    pub fn new(provider: GitProvider, ids: Vec<SqsRepoId>) -> Self {
        Self { provider, ids }
    }
}

#[async_trait]
impl TaskTrait for AddReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        for id in &self.ids {
            let clone_url = self.provider.repo(aws, id.repo_number()?).await?.clone_url;
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
