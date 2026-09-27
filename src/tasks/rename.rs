use crate::{
    aws::Aws,
    constants::S3_NAME_FILE,
    dto::sqs_event::{GitProvider, SqsRepoId},
    error::{ACError, ACResult},
    files::Files,
    s3::S3ObjectRef,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use serde::Serialize;
use std::time::SystemTime;

pub struct RenameTask {
    provider: GitProvider,
    id: SqsRepoId,
}

#[derive(Serialize)]
struct RenameBlob {
    account: String,
    repo: String,
}

impl RenameTask {
    pub fn new(provider: GitProvider, id: SqsRepoId) -> Self {
        Self { provider, id }
    }
}

#[async_trait]
impl TaskTrait for RenameTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let repository = self.provider.repo(aws, self.id.repo_number()?).await?;
        let refs = RepoRefs::new(&aws.config, &self.id)?;
        let key = format!("{}{S3_NAME_FILE}", RepoRefs::prefix(&self.id)?);
        let bucket = refs.repo_ref.bucket.clone();
        let refs_ref = &refs;
        let files = Files::new();
        aws.dynamodb
            .locked(&refs.lock_ref, deadline, move |lock| async move {
                files.clean()?;
                let body = serde_json::to_vec(&RenameBlob {
                    account: repository.owner,
                    repo: repository.name,
                })
                .map_err(|error| {
                    ACError::InternalServer(format!("Failed to serialize rename: {error}"))
                })?;
                aws.fenced_s3(&refs_ref.lock_ref, &lock)
                    .upload_bytes(&S3ObjectRef { bucket, key }, body)
                    .await
            })
            .await
    }
}
