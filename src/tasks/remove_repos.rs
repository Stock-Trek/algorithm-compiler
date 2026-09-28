use crate::{
    aws::Aws,
    error::ACResult,
    git_remote::GitRemote,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct RemoveReposTask {
    git_remote: GitRemote,
    repo_ids: Vec<String>,
}

impl RemoveReposTask {
    pub fn new(git_remote: GitRemote, repo_ids: Vec<String>) -> Self {
        Self {
            git_remote,
            repo_ids,
        }
    }
}

#[async_trait]
impl TaskTrait for RemoveReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        for repo_id in &self.repo_ids {
            let refs = RepoRefs::new(&aws.config, self.git_remote.clone(), repo_id)?;
            let prefix = RepoRefs::prefix(&self.git_remote, repo_id)?;
            let refs_ref = &refs;
            let prefix_ref = &prefix;
            aws.dynamodb
                .locked(&refs.lock_ref, deadline, move |lock| async move {
                    aws.fenced_s3(&refs_ref.lock_ref, &lock)
                        .delete_objects_with_prefix(&refs_ref.repo_ref.bucket, prefix_ref)
                        .await
                })
                .await?;
        }
        Ok(())
    }
}
