use crate::{
    aws::Aws,
    dto::sqs_event::SqsRepoId,
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddReposTask {
    ids: Vec<SqsRepoId>,
}

impl AddReposTask {
    pub fn new(ids: Vec<SqsRepoId>) -> Self {
        Self { ids }
    }
}

#[async_trait]
impl TaskTrait for AddReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        for id in &self.ids {
            let refs = RepoRefs::new(&aws.config, id)?;
            let repo = GitRepo::new(&id.clone_url, None, aws.config.timeouts);
            let files = Files::new();
            let refs_ref = &refs;
            let repo_ref = &repo;
            aws.dynamodb
                .locked(&refs.lock_ref, deadline, move |lock| async move {
                    refs_ref.sync(aws, files, repo_ref, &lock, deadline).await
                })
                .await?;
        }
        Ok(())
    }
}
