use crate::{
    aws::Aws,
    dto::sqs_event::{GitProvider, SqsRepoDetail},
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;

pub struct AddReposTask {
    provider: GitProvider,
    repos: Vec<SqsRepoDetail>,
}

impl AddReposTask {
    pub fn new(provider: GitProvider, repos: Vec<SqsRepoDetail>) -> Self {
        Self { provider, repos }
    }
}

#[async_trait]
impl TaskTrait for AddReposTask {
    async fn handle(&self, aws: &Aws) -> ACResult<()> {
        for detail in &self.repos {
            let refs = RepoRefs::new(&aws.config, detail)?;
            let repo = GitRepo::new(&self.provider, &detail.account, &detail.repo, None);
            let files = Files::new();
            aws.dynamodb
                .locked(&refs.lock_ref, || refs.sync(aws, &files, &repo))
                .await?;
        }
        Ok(())
    }
}
