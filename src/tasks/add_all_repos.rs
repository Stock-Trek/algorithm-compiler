use crate::{
    aws::Aws,
    error::ACResult,
    git_remote::GitRemote,
    tasks::{add_repos::AddReposTask, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddAllReposTask {
    git_remote: GitRemote,
}

impl AddAllReposTask {
    pub fn new(git_remote: GitRemote) -> Self {
        Self { git_remote }
    }
}

#[async_trait]
impl TaskTrait for AddAllReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let repo_ids = self.git_remote.account_repo_ids().await?;
        AddReposTask::new(self.git_remote.clone(), repo_ids)
            .handle(aws, deadline)
            .await
    }
}
