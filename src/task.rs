use crate::{
    archive::Archive,
    aws::Aws,
    dto::sqs_event::{SqsDetail, SqsMessage},
    dynamodb::DynamoDbDatumRef,
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    s3::S3ObjectRef,
};
use tracing::info;

const DYNAMODB_LOCK_TABLE: &str = "stock-trek-locks";
const DYNAMODB_LOCK_KEY_ATTRIBUTE: &str = "git-repository";
const S3_BUCKET_UPLOADS: &str = "stock-trek-uploads";

pub struct TaskOld {
    lock_ref: DynamoDbDatumRef,
    repo_ref: S3ObjectRef,
    files: Files,
    repo: GitRepo,
    task: SqsMessage,
}

impl TaskOld {
    pub async fn handle(&self, aws: &Aws) -> ACResult<()> {
        info!("Handle event for repo {:?}", self.repo);
        self.files.clean()?;
        let lock = aws.dynamodb.acquire_lock(&self.lock_ref).await?;
        let sync_result = self.sync(aws).await;
        let release_result = aws.dynamodb.release_lock(&self.lock_ref, &lock).await;
        sync_result?;
        release_result?;
        self.files.copy_algorithms().await?;
        self.files.compile()?;
        self.upload_artifacts(aws).await?;
        Ok(())
    }
}

impl TaskOld {
    async fn sync(&self, aws: &Aws) -> ACResult<()> {
        if aws.s3.download(&self.repo_ref, &self.files.archive).await? {
            Archive::extract(&self.files.archive, &self.files.repo)?;
            self.repo.fetch(&self.files.repo)?;
        } else {
            self.repo.clone_bare(&self.files.repo)?;
        }
        self.repo.create_ref(&self.files.repo)?;
        Archive::create(&self.files.repo, &self.files.archive)?;
        aws.s3.upload(&self.repo_ref, &self.files.archive).await?;
        Ok(())
    }

    async fn upload_artifacts(&self, aws: &Aws) -> ACResult<()> {
        info!("Upload artifacts to S3");
        let repo_id = self.repo.repo_id;
        let commit_hash = self.repo.commit_hash;
        let s3_key_raw_code = format!("repo-{}/hash-{}/algorithm.rs", repo_id, commit_hash);
        let s3_key_metadata = format!("repo-{}/hash-{}/metadata.rs", repo_id, commit_hash);
        let s3_key_cwasm = format!("repo-{}/hash-{}/binary.cwasm", repo_id, commit_hash);
        S3::upload(
            &clients.s3,
            S3_BUCKET_UPLOADS,
            &s3_key_raw_code,
            Path::new(TMP_ALGORITHM_RS),
        )
        .await?;
        if Path::new(TMP_METADATA_RS).exists() {
            S3::upload(
                &clients.s3,
                S3_BUCKET_UPLOADS,
                &s3_key_metadata,
                Path::new(TMP_METADATA_RS),
            )
            .await?;
        }
        S3::upload(
            &clients.s3,
            S3_BUCKET_UPLOADS,
            &s3_key_cwasm,
            Path::new(BUILT_CWASM),
        )
        .await?;
        Ok(())
    }
}

impl From<SqsMessage> for TaskOld {
    fn from(value: SqsMessage) -> Self {
        let SqsMessage { provider, detail } = value;
        match detail {
            SqsDetail::AddRepos { ids } => {}
            SqsDetail::Commit {
                ids,
                branch_name,
                commit_hash,
                forced,
            } => {}
            SqsDetail::RemoveRepos { ids } => {}
            SqsDetail::Rename { ids, names } => {}
        }
        let branch_name = r#ref
            .strip_prefix("refs/heads/")
            .unwrap_or(&r#ref)
            .to_string();
        let sanitized_repo_name = Files::sanitize_path(&repo);
        let s3_object_ref = S3ObjectRef {
            bucket: S3_BUCKET_UPLOADS.into(),
            key: format!("repos/{}.tar.gz", &sanitized_repo_name),
        };
        let dynamodb_datum_ref = DynamoDbDatumRef {
            key_name: DYNAMODB_LOCK_KEY_ATTRIBUTE.into(),
            key_value: sanitized_repo_name,
            table: DYNAMODB_LOCK_TABLE.into(),
        };
        let files = Files::new();
        let ref_name = format!("refs/stock-trek/{branch_name}");
        let repo = GitRepo {
            clone_url,
            repo_id,
            full_name,
            name,
            ref_name,
            branch_name,
            commit_hash: after,
        };
        Self {
            lock_ref: dynamodb_datum_ref,
            repo_ref: s3_object_ref,
            files,
            repo,
        }
    }
}
