use crate::{
    aws::Aws,
    dto::sqs_event::GitSource,
    error::ACResult,
    files::Files,
    tasks::{remove_repos::RemoveReposTask, task::TaskTrait},
};
use async_trait::async_trait;
use std::{collections::BTreeSet, time::SystemTime};

pub struct RemoveAllReposTask {
    source: GitSource,
}

impl RemoveAllReposTask {
    pub fn new(source: GitSource) -> Self {
        Self { source }
    }
}

#[async_trait]
impl TaskTrait for RemoveAllReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let account = Files::sanitize_path(&self.source.account_id())?;
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
        RemoveReposTask::new(self.source.clone(), repo_ids.into_iter().collect())
            .handle(aws, deadline)
            .await
    }
}
