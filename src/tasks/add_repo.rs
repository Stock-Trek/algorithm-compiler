use crate::{
    aws::Aws,
    error::ACResult,
    git_remote::GitRemote,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddRepoTask {
    git_remote: GitRemote,
    repo_id: String,
}

impl AddRepoTask {
    pub fn new(git_remote: GitRemote, repo_id: String) -> Self {
        Self {
            git_remote,
            repo_id,
        }
    }
}

#[async_trait]
impl TaskTrait for AddRepoTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let refs = RepoRefs::new(&aws.config, self.git_remote.clone(), &self.repo_id)?;
        let refs_ref = &refs;
        aws.dynamodb
            .locked(&refs.lock_ref, deadline, move |lock| async move {
                refs_ref.sync(aws, &lock, deadline).await
            })
            .await
    }
}
