use crate::{
    archive::Archive,
    aws::Aws,
    config::Config,
    constants::{DYNAMODB_LOCK_KEY_ATTRIBUTE, S3_REPOS_PREFIX},
    dto::sqs_event::GitSource,
    dynamodb::{DynamoDbDatumRef, DynamoDbLock},
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    s3::{DownloadOutcome, S3ObjectRef},
};
use std::time::SystemTime;

pub struct RepoRefs {
    pub lock_ref: DynamoDbDatumRef,
    pub repo_ref: S3ObjectRef,
}

impl RepoRefs {
    pub fn new(config: &Config, source: GitSource) -> ACResult<Self> {
        let account = Files::sanitize_path(&source.account_id)?;
        let repo = Files::sanitize_path(&source.repo_id)?;
        Ok(Self {
            lock_ref: DynamoDbDatumRef {
                table: config.dynamodb_lock_table.clone(),
                key_name: DYNAMODB_LOCK_KEY_ATTRIBUTE.into(),
                key_value: format!("{account}/{repo}"),
            },
            repo_ref: S3ObjectRef {
                bucket: config.s3_bucket_commit_artifacts.clone(),
                key: format!("{account}/{repo}/{S3_REPOS_PREFIX}/{repo}.tar.gz"),
            },
        })
    }

    pub fn prefix(repo_id: &str) -> ACResult<String> {
        let account = Files::sanitize_path(&id.account_id)?;
        let repo = Files::sanitize_path(&id.repo_id)?;
        Ok(format!("{account}/{repo}/"))
    }

    pub async fn sync(
        &self,
        clone_url: &str,
        commit: Option<(&str, &str)>,
        aws: &Aws,
        lock: &DynamoDbLock,
        deadline: SystemTime,
    ) -> ACResult<()> {
        let files = Files::new();
        let repo = GitRepo::new(aws.config.timeouts);
        files.prepare()?;
        match aws.s3.download(&self.repo_ref, &files.archive).await? {
            DownloadOutcome::Downloaded => {
                Archive::extract(&files.archive, &files.repo)?;
                repo.fetch(&files.repo, deadline).await?;
            }
            DownloadOutcome::NotFound => {
                repo.clone_bare(clone_url, &files.repo, deadline).await?;
            }
        }
        if let Some((ref_name, commit_hash)) = commit {
            repo.create_ref(ref_name, commit_hash, &files.repo, deadline)
                .await?;
        }
        Archive::create(&files.repo, &files.archive)?;
        aws.fenced_s3(&self.lock_ref, lock)
            .upload(&self.repo_ref, &files.archive)
            .await?;
        Ok(())
    }
}
