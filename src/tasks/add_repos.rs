use crate::{
    aws::Aws,
    dto::sqs_event::{GitProvider, SqsRepoDetail},
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddReposTask {
    provider: GitProvider,
    ids: Vec<SqsRepoDetail>,
}

impl AddReposTask {
    pub fn new(provider: GitProvider, ids: Vec<SqsRepoDetail>) -> Self {
        Self { provider, ids }
    }
}

#[async_trait]
impl TaskTrait for AddReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        for detail in &self.ids {
            let refs = RepoRefs::new(&aws.config, detail)?;
            let repo = GitRepo::new(&self.provider, &detail.account, &detail.repo, None);
            let files = Files::new();
            aws.dynamodb
                .locked(&refs.lock_ref, deadline, || refs.sync(aws, &files, &repo))
                .await?;
        }
        Ok(())
    }
}
