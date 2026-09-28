use crate::{
    aws::Aws,
    error::ACResult,
    git_remote::GitRemote,
    tasks::{remove_repos::RemoveReposTask, task::TaskTrait},
};
use async_trait::async_trait;
use std::{collections::BTreeSet, time::SystemTime};

pub struct RemoveAllReposTask {
    git_remote: GitRemote,
}

impl RemoveAllReposTask {
    pub fn new(git_remote: GitRemote) -> Self {
        Self { git_remote }
    }
}

#[async_trait]
impl TaskTrait for RemoveAllReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let keys = aws
            .s3
            .list_keys_with_prefix(&aws.config.s3_bucket_commit_artifacts, "")
            .await?;
        let repo_ids: BTreeSet<String> = keys
            .iter()
            .filter_map(|key| key.split_once('/').map(|(repo, _)| repo.to_string()))
            .collect();
        RemoveReposTask::new(self.git_remote.clone(), repo_ids.into_iter().collect())
            .handle(aws, deadline)
            .await
    }
}
