use crate::{
    aws::Aws,
    error::ACResult,
    git_remote::GitRemote,
    tasks::{connector_refs::ConnectorRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct RepoAddTask {
    git_remote: GitRemote,
    repo_id: String,
}

impl RepoAddTask {
    pub fn new(git_remote: GitRemote, repo_id: String) -> Self {
        Self {
            git_remote,
            repo_id,
        }
    }
}

#[async_trait]
impl TaskTrait for RepoAddTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let refs = ConnectorRefs::new(&aws.config, self.git_remote.clone(), &self.repo_id)?;
        let refs_ref = &refs;
        aws.dynamodb
            .locked(&refs.lock_ref, deadline, move |lock| async move {
                refs_ref.sync(aws, &lock, deadline).await
            })
            .await
    }
}
