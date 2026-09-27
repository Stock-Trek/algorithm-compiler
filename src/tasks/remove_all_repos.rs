use crate::{
    aws::Aws,
    dto::sqs_event::{GitProvider, SqsRepoId},
    error::ACResult,
    files::Files,
    tasks::{remove_repos::RemoveReposTask, task::TaskTrait},
};
use async_trait::async_trait;
use std::{collections::BTreeSet, time::SystemTime};

pub struct RemoveAllReposTask {
    provider: GitProvider,
    account_id: String,
}

impl RemoveAllReposTask {
    pub fn new(provider: GitProvider, account_id: String) -> Self {
        Self {
            provider,
            account_id,
        }
    }
}

#[async_trait]
impl TaskTrait for RemoveAllReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let prefix = format!("{}/", Files::sanitize_path(&self.account_id)?);
        let keys = aws
            .s3
            .list_keys_with_prefix(&aws.config.s3_bucket_commit_artifacts, &prefix)
            .await?;
        let repo_ids: BTreeSet<String> = keys
            .iter()
            .filter_map(|key| key.strip_prefix(&prefix))
            .filter_map(|path| path.split_once('/').map(|(repo, _)| repo.to_string()))
            .collect();
        let ids = repo_ids
            .into_iter()
            .map(|repo_id| SqsRepoId {
                account_id: self.account_id.clone(),
                repo_id,
            })
            .collect();
        RemoveReposTask::new(self.provider.clone(), ids)
            .handle(aws, deadline)
            .await
    }
}
