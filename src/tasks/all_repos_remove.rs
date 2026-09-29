use crate::{
    aws::Aws,
    dto::sqs_event::GitSource,
    error::ACResult,
    git_remote::GitRemote,
    tasks::{multi_repo_remove::MultiRepoRemoveTask, task::TaskTrait},
};
use async_trait::async_trait;
use std::{collections::BTreeSet, time::SystemTime};

pub struct AllReposRemoveTask {
    source: GitSource,
    git_remote: GitRemote,
}

impl AllReposRemoveTask {
    pub fn new(source: GitSource, git_remote: GitRemote) -> Self {
        Self { source, git_remote }
    }
}

#[async_trait]
impl TaskTrait for AllReposRemoveTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let keys = aws
            .s3
            .list_keys_with_prefix(&aws.config.s3_bucket_commit_artifacts, "")
            .await?;
        let provider_prefix = format!("{}/", self.git_remote.provider());
        let repo_ids: BTreeSet<String> = keys
            .iter()
            .filter_map(|key| key.strip_prefix(&provider_prefix))
            .filter_map(|key| key.split_once('/').map(|(repo, _)| repo.to_string()))
            .collect();
        MultiRepoRemoveTask::new(self.source.clone(), repo_ids.into_iter().collect())
            .handle(aws, deadline)
            .await
    }
}
