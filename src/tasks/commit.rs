use crate::{
    archive::Archive,
    aws::Aws,
    constants::{S3_COMPILE_OUTPUT_FILE, S3_COMPILE_RESULT_FILE},
    dto::{
        compile_result::CompileResult,
        sqs_event::{GitProvider, SqsRepoDetail},
    },
    error::{ACError, ACResult},
    files::{ALGORITHMS_ARCHIVE_FILE, Files},
    git_repo::GitRepo,
    s3::S3ObjectRef,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::{path::Path, time::SystemTime};

pub struct CommitTask {
    provider: GitProvider,
    ids: SqsRepoDetail,
    branch_name: String,
    commit_hash: String,
}

impl CommitTask {
    pub fn new(
        provider: GitProvider,
        ids: SqsRepoDetail,
        branch_name: String,
        commit_hash: String,
    ) -> Self {
        Self {
            provider,
            ids,
            branch_name,
            commit_hash,
        }
    }

    fn prefix(&self) -> ACResult<String> {
        Ok(format!(
            "{}/{}/{}",
            Files::sanitize_path(&self.ids.account)?,
            Files::sanitize_path(&self.ids.repo)?,
            Files::sanitize_path(&self.commit_hash)?
        ))
    }

    async fn upload_artifacts(&self, aws: &Aws, files: &Files) -> ACResult<()> {
        let prefix = self.prefix()?;
        Archive::create(&files.algorithms, &files.algorithms_archive)?;
        self.upload(
            aws,
            &format!("{prefix}/{ALGORITHMS_ARCHIVE_FILE}"),
            &files.algorithms_archive,
        )
        .await?;
        self.upload(aws, &format!("{prefix}/binary.cwasm"), &files.cwasm_file())
            .await
    }

    async fn upload_compile_output(
        &self,
        aws: &Aws,
        compile_result: &CompileResult,
    ) -> ACResult<()> {
        let body = serde_json::to_vec(compile_result).map_err(|error| {
            ACError::InternalServer(format!("Failed to serialize compile result: {error}"))
        })?;
        aws.s3
            .upload_bytes(
                &S3ObjectRef {
                    bucket: aws.config.s3_bucket_commit_artifacts.clone(),
                    key: format!("{}/{S3_COMPILE_RESULT_FILE}", self.prefix()?),
                },
                body,
            )
            .await
    }

    async fn upload_raw_compile_output(&self, aws: &Aws, files: &Files) -> ACResult<()> {
        let path = files.compile_output_file();
        if !path.exists() {
            return Ok(());
        }
        self.upload(
            aws,
            &format!("{}/{S3_COMPILE_OUTPUT_FILE}", self.prefix()?),
            &path,
        )
        .await
    }

    async fn upload(&self, aws: &Aws, key: &str, path: &Path) -> ACResult<()> {
        aws.s3
            .upload(
                &S3ObjectRef {
                    bucket: aws.config.s3_bucket_commit_artifacts.clone(),
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
        let refs = RepoRefs::new(&aws.config, &self.ids)?;
        let repo = GitRepo::new(
            &self.provider,
            &self.ids.account,
            &self.ids.repo,
            Some((&self.branch_name, &self.commit_hash)),
        );
        let files = Files::new();
        aws.dynamodb
            .locked(&refs.lock_ref, deadline, || async {
                refs.sync(aws, files, &repo).await?;
                files.copy_algorithms(&self.commit_hash).await?;
                let compile_result = files.compile().await;
                self.upload_raw_compile_output(aws, files).await?;
                let compile_result = compile_result?;
                self.upload_compile_output(aws, &compile_result).await?;
                if compile_result.failed() {
                    return Ok(());
                }
                self.upload_artifacts(aws, files).await
            })
            .await
    }
}
