use crate::{
    aws::Aws,
    constants::S3_NAME_FILE,
    error::{ACError, ACResult},
    files::Files,
    git_remote::GitRemote,
    s3::S3ObjectRef,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use serde::Serialize;
use std::time::SystemTime;

pub struct RenameTask {
    git_remote: GitRemote,
    repo_id: String,
}

#[derive(Serialize)]
struct NameBlob {
    account: String,
    repo: String,
}

impl RenameTask {
    pub fn new(git_remote: GitRemote, repo_id: String) -> Self {
        Self {
            git_remote,
            repo_id,
        }
    }
}

#[async_trait]
impl TaskTrait for RenameTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let (account_name, repo_name) = self.git_remote.account_repo_name(&self.repo_id).await?;
        let key = format!(
            "{}{S3_NAME_FILE}",
            RepoRefs::prefix(&self.git_remote, &self.repo_id)?
        );
        let refs = RepoRefs::new(&aws.config, self.git_remote.clone(), &self.repo_id)?;
        let bucket = refs.repo_ref.bucket.clone();
        let refs_ref = &refs;
        let files = Files::new();
        aws.dynamodb
            .locked(&refs.lock_ref, deadline, move |lock| async move {
                files.clean()?;
                let blob = NameBlob {
                    account: account_name,
                    repo: repo_name,
                };
                let body = serde_json::to_vec(&blob).map_err(|error| {
                    ACError::InternalServer(format!("Failed to serialize rename: {error}"))
                })?;
                aws.fenced_s3(&refs_ref.lock_ref, &lock)
                    .upload_bytes(&S3ObjectRef { bucket, key }, body)
                    .await
            })
            .await
    }
}
