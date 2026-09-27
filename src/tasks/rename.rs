use crate::{
    aws::Aws,
    dto::sqs_event::{GitProvider, SqsRepoId},
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct RenameTask {
    provider: GitProvider,
    id: SqsRepoId,
}

impl RenameTask {
    pub fn new(provider: GitProvider, id: SqsRepoId) -> Self {
        Self { provider, id }
    }
}

#[async_trait]
impl TaskTrait for RenameTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let refs = RepoRefs::new(&aws.config, &self.id)?;
        let repo = GitRepo::new(aws.config.timeouts);
        let refs_ref = &refs;
        let files = Files::new();
        aws.dynamodb
            .locked(&refs.lock_ref, deadline, move |lock| async move {
                files.clean()?;
                // TODO write a json blob containing new account/repo name and upload it to (fenced) s3
                Ok(())
            })
            .await
    }
}
