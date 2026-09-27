use crate::{
    aws::Aws,
    error::ACResult,
    files::Files,
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
        let account_id = self.git_remote.account_id().await?;
        let account = Files::sanitize_path(&account_id)?;
        let prefix = format!("{account}/");
        let keys = aws
            .s3
            .list_keys_with_prefix(&aws.config.s3_bucket_commit_artifacts, &prefix)
            .await?;
        let repo_ids: BTreeSet<String> = keys
            .iter()
            .filter_map(|key| key.strip_prefix(&prefix))
            .filter_map(|path| path.split_once('/').map(|(repo, _)| repo.to_string()))
            .collect();
        RemoveReposTask::new(self.git_remote.clone(), repo_ids.into_iter().collect())
            .handle(aws, deadline)
            .await
    }
}
