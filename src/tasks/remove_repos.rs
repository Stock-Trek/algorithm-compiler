use crate::{aws::Aws, dto::sqs_event::SqsRepoDetail, error::ACResult, tasks::task::TaskTrait};
use async_trait::async_trait;

pub struct RemoveReposTask {
    pub repos: Vec<SqsRepoDetail>,
}

#[async_trait]
impl TaskTrait for RemoveReposTask {
    async fn handle(&self, aws: &Aws) -> ACResult<()> {
        for repo in &self.repos {
            match self.remove_repo(repo, aws).await {
                Err(_) => {
                    eprintln!("Failed to remove repo: {:?}", repo);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl RemoveReposTask {
    async fn remove_repo(&self, repo: &SqsRepoDetail, aws: &Aws) -> ACResult<()> {
        let lock_ref = self.dynamodb_lock_ref(repo);
        let repo_prefix = self.repo_prefix(repo);
        let lock = aws.dynamodb.acquire_lock(&lock_ref).await?;
        let delete_result = aws
            .s3
            .delete_objects_with_prefix(Self::S3_BUCKET_UPLOADS, &repo_prefix)
            .await;
        let release_result = aws.dynamodb.release_lock(&lock_ref, &lock).await;
        delete_result?;
        release_result?;
        Ok(())
    }
}
