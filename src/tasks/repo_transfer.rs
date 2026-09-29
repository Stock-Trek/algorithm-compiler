use crate::{aws::Aws, error::ACResult, git_remote::GitRemote, tasks::task::TaskTrait};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct RepoTransferTask {
    git_remote: GitRemote,
    repo_id: String,
}

impl RepoTransferTask {
    pub fn new(git_remote: GitRemote, repo_id: String) -> Self {
        Self {
            git_remote,
            repo_id,
        }
    }
}

#[async_trait]
impl TaskTrait for RepoTransferTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        Ok(())
    }
}
