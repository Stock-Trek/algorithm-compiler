use crate::{
    archive::Archive,
    aws::Aws,
    config::Config,
    constants::{DYNAMODB_LOCK_KEY_ATTRIBUTE, S3_REPOS_PREFIX},
    dto::sqs_event::SqsRepoDetail,
    dynamodb::DynamoDbDatumRef,
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    s3::S3ObjectRef,
};

pub struct RepoRefs {
    pub lock_ref: DynamoDbDatumRef,
    pub repo_ref: S3ObjectRef,
}

impl RepoRefs {
    pub fn new(config: &Config, detail: &SqsRepoDetail) -> Self {
        let account = Files::sanitize_path(&detail.account);
        let repo = Files::sanitize_path(&detail.repo);
        Self {
            lock_ref: DynamoDbDatumRef {
                table: config.dynamodb_lock_table.clone(),
                key_name: DYNAMODB_LOCK_KEY_ATTRIBUTE.into(),
                key_value: format!("{account}/{repo}"),
            },
            repo_ref: S3ObjectRef {
                bucket: config.s3_bucket_uploads.clone(),
                key: format!("{account}/{repo}/{S3_REPOS_PREFIX}/{repo}.tar.gz"),
            },
        }
    }

    pub fn prefix(detail: &SqsRepoDetail) -> String {
        let account = Files::sanitize_path(&detail.account);
        let repo = Files::sanitize_path(&detail.repo);
        format!("{account}/{repo}/")
    }

    pub async fn sync(&self, aws: &Aws, files: &Files, repo: &GitRepo) -> ACResult<()> {
        files.prepare()?;
        if aws.s3.download(&self.repo_ref, &files.archive).await? {
            Archive::extract(&files.archive, &files.repo)?;
            repo.fetch(&files.repo)?;
        } else {
            repo.clone_bare(&files.repo)?;
        }
        repo.create_ref(&files.repo)?;
        Archive::create(&files.repo, &files.archive)?;
        aws.s3.upload(&self.repo_ref, &files.archive).await?;
        Ok(())
    }
}
