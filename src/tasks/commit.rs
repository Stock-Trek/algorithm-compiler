use crate::{
    archive::Archive,
    aws::Aws,
    constants::{S3_COMPILE_OUTPUT_FILE, S3_COMPILE_RESULT_FILE},
    dto::compile_result::{CompileResult, CompileStatus},
    error::{ACError, ACResult},
    fenced::FencedS3,
    files::{ALGORITHMS_ARCHIVE_FILE, Files},
    git_local::GitLocal,
    git_remote::GitRemote,
    s3::S3ObjectRef,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::{path::Path, time::SystemTime};

pub struct CommitTask {
    git_remote: GitRemote,
    repo_id: String,
    branch_name: String,
    commit_hash: String,
}

impl CommitTask {
    pub fn new(
        git_remote: GitRemote,
        repo_id: String,
        branch_name: String,
        commit_hash: String,
    ) -> Self {
        Self {
            git_remote,
            repo_id,
            branch_name,
            commit_hash,
        }
    }

    fn prefix(&self) -> ACResult<String> {
        let base = RepoRefs::prefix(&self.git_remote, &self.repo_id)?;
        Ok(format!(
            "{base}{}",
            Files::sanitize_path(&self.commit_hash)?
        ))
    }

    async fn upload_artifacts(
        &self,
        bucket: &str,
        s3: &FencedS3<'_>,
        files: &Files,
    ) -> ACResult<()> {
        let prefix = self.prefix()?;
        Archive::create(&files.algorithms, &files.algorithms_archive).await?;
        self.upload(
            bucket,
            s3,
            &format!("{prefix}/{ALGORITHMS_ARCHIVE_FILE}"),
            &files.algorithms_archive,
        )
        .await?;
        self.upload(
            bucket,
            s3,
            &format!("{prefix}/binary.cwasm"),
            &files.cwasm_file(),
        )
        .await
    }

    async fn upload_compile_output(
        &self,
        bucket: &str,
        s3: &FencedS3<'_>,
        compile_result: &CompileResult,
    ) -> ACResult<()> {
        let body = serde_json::to_vec(compile_result).map_err(|error| {
            ACError::InternalServer(format!("Failed to serialize compile result: {error}"))
        })?;
        s3.upload_bytes(
            &S3ObjectRef {
                bucket: bucket.into(),
                key: format!("{}/{S3_COMPILE_RESULT_FILE}", self.prefix()?),
            },
            body,
        )
        .await
    }

    async fn upload_raw_compile_output(
        &self,
        bucket: &str,
        s3: &FencedS3<'_>,
        files: &Files,
    ) -> ACResult<()> {
        let path = files.compile_output_file();
        if !tokio::fs::try_exists(&path)
            .await
            .map_err(ACError::FileSystem)?
        {
            return Ok(());
        }
        self.upload(
            bucket,
            s3,
            &format!("{}/{S3_COMPILE_OUTPUT_FILE}", self.prefix()?),
            &path,
        )
        .await
    }

    async fn upload(
        &self,
        bucket: &str,
        s3: &FencedS3<'_>,
        key: &str,
        path: &Path,
    ) -> ACResult<()> {
        s3.upload(
            &S3ObjectRef {
                bucket: bucket.into(),
                key: key.into(),
            },
            path,
        )
        .await
    }
}

#[async_trait]
impl TaskTrait for CommitTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let ref_name = GitLocal::stock_trek_ref_name(&self.branch_name, &self.commit_hash);
        let refs = RepoRefs::new(&aws.config, self.git_remote.clone(), &self.repo_id)?;
        let refs_ref = &refs;
        aws.dynamodb
            .locked(&refs.lock_ref, deadline, move |lock| async move {
                refs_ref
                    .sync_commit(&ref_name, &self.commit_hash, aws, &lock, deadline)
                    .await?;
                let files = Files::new();
                files
                    .copy_algorithms(&self.commit_hash, &aws.config.timeouts, deadline)
                    .await?;
                let compile_result = files.compile(&aws.config.timeouts, deadline).await?;
                let s3 = aws.fenced_s3(&refs_ref.lock_ref, &lock);
                let bucket = &aws.config.s3_bucket_commit_artifacts;
                self.upload_compile_output(bucket, &s3, &compile_result)
                    .await?;
                if compile_result.result != CompileStatus::Success {
                    self.upload_raw_compile_output(bucket, &s3, files).await?;
                    return Ok(());
                }
                self.upload_artifacts(bucket, &s3, files).await
            })
            .await
    }
}
