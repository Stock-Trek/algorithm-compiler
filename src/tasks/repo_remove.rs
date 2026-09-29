use crate::{
    aws::Aws,
    error::ACResult,
    git_remote::GitRemote,
    tasks::{connector_refs::ConnectorRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct RepoRemoveTask {
    git_remote: GitRemote,
    repo_id: String,
}

impl RepoRemoveTask {
    pub fn new(git_remote: GitRemote, repo_id: String) -> Self {
        Self {
            git_remote,
            repo_id,
        }
    }
}

#[async_trait]
impl TaskTrait for RepoRemoveTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let refs = ConnectorRefs::new(&aws.config, self.git_remote.clone(), &self.repo_id)?;
        let prefix = ConnectorRefs::prefix(&self.git_remote, &self.repo_id)?;
        let refs_ref = &refs;
        let prefix_ref = &prefix;
        aws.dynamodb
            .locked(&refs.lock_ref, deadline, move |lock| async move {
                aws.fenced_s3(&refs_ref.lock_ref, &lock)
                    .delete_objects_with_prefix(&refs_ref.repo_ref.bucket, prefix_ref)
                    .await
            })
            .await
    }
}
