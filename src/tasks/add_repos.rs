use crate::{
    aws::Aws,
    error::ACResult,
    git_remote::GitRemote,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddReposTask {
    git_remote: GitRemote,
    repo_ids: Vec<String>,
}

impl AddReposTask {
    pub fn new(git_remote: GitRemote, repo_ids: Vec<String>) -> Self {
        Self {
            git_remote,
            repo_ids,
        }
    }
}

#[async_trait]
impl TaskTrait for AddReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        for repo_id in &self.repo_ids {
            let refs = RepoRefs::new(&aws.config, self.git_remote.clone(), repo_id)?;
            let refs_ref = &refs;
            aws.dynamodb
                .locked(&refs.lock_ref, deadline, move |lock| async move {
                    refs_ref.sync(aws, &lock, deadline).await
                })
                .await?;
        }
        Ok(())
    }
}
