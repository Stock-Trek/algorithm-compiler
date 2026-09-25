use crate::{
    archive::Archive,
    aws::Aws,
    constants::{
        DYNAMODB_LOCK_KEY_ATTRIBUTE, DYNAMODB_LOCK_TABLE, S3_BUCKET_UPLOADS, S3_REPOS_PREFIX,
    },
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
    pub fn new(detail: &SqsRepoDetail) -> Self {
        let account = Files::sanitize_path(&detail.account);
        let repo = Files::sanitize_path(&detail.repo);
        Self {
            lock_ref: DynamoDbDatumRef {
                table: DYNAMODB_LOCK_TABLE.into(),
                key_name: DYNAMODB_LOCK_KEY_ATTRIBUTE.into(),
                key_value: format!("{account}/{repo}"),
            },
            repo_ref: S3ObjectRef {
                bucket: S3_BUCKET_UPLOADS.into(),
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
