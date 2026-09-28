use crate::{
    archive::Archive,
    aws::Aws,
    config::Config,
    constants::{DYNAMODB_LOCK_KEY_ATTRIBUTE, S3_REPOS_PREFIX},
    dynamodb::{DynamoDbDatumRef, DynamoDbLock},
    error::ACResult,
    files::Files,
    git_local::GitLocal,
    git_remote::GitRemote,
    s3::{DownloadOutcome, S3ObjectRef},
};
use std::time::SystemTime;

pub struct RepoRefs {
    pub lock_ref: DynamoDbDatumRef,
    pub repo_ref: S3ObjectRef,
    git_remote: GitRemote,
    repo_id: String,
}

impl RepoRefs {
    pub fn new(config: &Config, git_remote: GitRemote, repo_id: &str) -> ACResult<Self> {
        let repo = Files::sanitize_path(repo_id)?;
        Ok(Self {
            lock_ref: DynamoDbDatumRef {
                table: config.dynamodb_lock_table.clone(),
                key_name: DYNAMODB_LOCK_KEY_ATTRIBUTE.into(),
                key_value: repo.clone(),
            },
            repo_ref: S3ObjectRef {
                bucket: config.s3_bucket_commit_artifacts.clone(),
                key: format!("{repo}/{S3_REPOS_PREFIX}/{repo}.tar.gz"),
            },
            git_remote,
            repo_id: repo_id.to_string(),
        })
    }

    pub fn prefix(repo_id: &str) -> ACResult<String> {
        let repo = Files::sanitize_path(repo_id)?;
        Ok(format!("{repo}/"))
    }

    pub async fn sync(&self, aws: &Aws, lock: &DynamoDbLock, deadline: SystemTime) -> ACResult<()> {
        self.prepare_repo(aws, deadline).await?;
        self.archive_and_upload(aws, lock).await
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

    async fn prepare_repo(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let files = Files::new();
        files.prepare()?;
        match aws.s3.download(&self.repo_ref, &files.archive).await? {
            DownloadOutcome::Downloaded => {
                Archive::extract(&files.archive, &files.repo)?;
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
        Archive::create(&files.repo, &files.archive)?;
        aws.fenced_s3(&self.lock_ref, lock)
            .upload(&self.repo_ref, &files.archive)
            .await
    }
}
