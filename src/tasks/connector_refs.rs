use crate::{
    archive::Archive,
    aws::Aws,
    config::Config,
    constants::{DYNAMODB_LOCK_KEY_ATTRIBUTE, S3_NAME_FILE, S3_REPOS_PREFIX},
    dynamodb::{DynamoDbDatumRef, DynamoDbLock},
    error::{ACError, ACResult},
    files::Files,
    git_local::GitLocal,
    git_remote::GitRemote,
    s3::{DownloadOutcome, S3ObjectRef},
};
use serde::Serialize;
use std::{path::Path, time::SystemTime};

#[derive(Serialize)]
struct NameBlob {
    account: String,
    repo: String,
}

pub struct ConnectorRefs {
    pub lock_ref: DynamoDbDatumRef,
    pub repo_ref: S3ObjectRef,
    git_remote: GitRemote,
    repo_id: String,
}

impl ConnectorRefs {
    pub fn new(config: &Config, git_remote: GitRemote, repo_id: &str) -> ACResult<Self> {
        let repo = Files::sanitize_path(repo_id)?;
        let path = format!("{}/{repo}", git_remote.provider());
        Ok(Self {
            lock_ref: DynamoDbDatumRef {
                table: config.dynamodb_lock_table.clone(),
                key_name: DYNAMODB_LOCK_KEY_ATTRIBUTE.into(),
                key_value: path.clone(),
            },
            repo_ref: S3ObjectRef {
                bucket: config.s3_bucket_commit_artifacts.clone(),
                key: format!("{path}/{S3_REPOS_PREFIX}/{repo}.tar.gz"),
            },
            git_remote,
            repo_id: repo_id.to_string(),
        })
    }

    pub fn prefix(git_remote: &GitRemote, repo_id: &str) -> ACResult<String> {
        let repo = Files::sanitize_path(repo_id)?;
        Ok(format!("{}/{repo}/", git_remote.provider()))
    }

    pub async fn sync(&self, aws: &Aws, lock: &DynamoDbLock, deadline: SystemTime) -> ACResult<()> {
        self.prepare_repo(aws, deadline).await?;
        self.archive_and_upload(aws, lock).await
    }

    pub async fn update_name(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let (account, repo) = self.git_remote.account_repo_name(&self.repo_id).await?;
        aws.dynamodb
            .locked(&self.lock_ref, deadline, move |lock| async move {
                Files::new().clean().await?;
                self.write_name(&account, &repo, aws, &lock).await
            })
            .await
    }

    async fn write_name(
        &self,
        account: &str,
        repo: &str,
        aws: &Aws,
        lock: &DynamoDbLock,
    ) -> ACResult<()> {
        let key = format!(
            "{}{S3_NAME_FILE}",
            Self::prefix(&self.git_remote, &self.repo_id)?
        );
        let body = serde_json::to_vec(&NameBlob {
            account: account.to_string(),
            repo: repo.to_string(),
        })
        .map_err(|error| ACError::InternalServer(format!("Failed to serialize name: {error}")))?;
        aws.fenced_s3(&self.lock_ref, lock)
            .upload_bytes(
                &S3ObjectRef {
                    bucket: self.repo_ref.bucket.clone(),
                    key,
                },
                body,
            )
            .await
    }

    pub async fn sync_commit(
        &self,
        ref_name: &str,
        commit_hash: &str,
        aws: &Aws,
        lock: &DynamoDbLock,
        deadline: SystemTime,
    ) -> ACResult<()> {
        self.prepare_repo(aws, deadline).await?;
        let files = Files::new();
        GitLocal
            .create_ref(
                aws.config.timeouts,
                ref_name,
                commit_hash,
                &files.repo,
                deadline,
            )
            .await?;
        self.archive_and_upload(aws, lock).await
    }

    pub async fn set_remote(
        &self,
        aws: &Aws,
        repo_dir: &Path,
        deadline: SystemTime,
    ) -> ACResult<()> {
        self.git_remote
            .set_remote(aws.config.timeouts, &self.repo_id, repo_dir, deadline)
            .await
    }

    async fn prepare_repo(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let files = Files::new();
        files.prepare().await?;
        match aws.s3.download(&self.repo_ref, &files.archive).await? {
            DownloadOutcome::Downloaded => {
                Archive::extract(&files.archive, &files.repo).await?;
                self.set_remote(aws, &files.repo, deadline).await?;
                GitLocal
                    .fetch(aws.config.timeouts, &files.repo, deadline)
                    .await?;
            }
            DownloadOutcome::NotFound => {
                self.git_remote
                    .clone_bare_repo(
                        aws.config.timeouts,
                        &self.repo_id,
                        Files::path_str(&files.repo)?,
                        deadline,
                    )
                    .await?;
            }
        }
        Ok(())
    }

    async fn archive_and_upload(&self, aws: &Aws, lock: &DynamoDbLock) -> ACResult<()> {
        let files = Files::new();
        Archive::create(&files.repo, &files.archive).await?;
        aws.fenced_s3(&self.lock_ref, lock)
            .upload(&self.repo_ref, &files.archive)
            .await
    }
}
