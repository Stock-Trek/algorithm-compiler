use crate::{
    aws::Aws,
    constants::{S3_BUCKET_UPLOADS, S3_UPLOADS_PREFIX},
    dto::sqs_event::SqsRepoDetail,
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    s3::S3ObjectRef,
    tasks::{
        repo_refs::{RepoRefs, locked, sync_repo},
        task::TaskTrait,
    },
};
use async_trait::async_trait;
use std::path::Path;

pub struct CommitTask {
    repo: SqsRepoDetail,
    branch_name: String,
    commit_hash: String,
}

impl CommitTask {
    pub fn new(repo: SqsRepoDetail, branch_name: String, commit_hash: String) -> Self {
        Self {
            repo,
            branch_name,
            commit_hash,
        }
    }

    async fn upload_artifacts(&self, aws: &Aws, files: &Files) -> ACResult<()> {
        let prefix = format!(
            "{S3_UPLOADS_PREFIX}/{}/{}/{}",
            Files::sanitize_path(&self.repo.account),
            Files::sanitize_path(&self.repo.repo),
            self.commit_hash
        );
        self.upload(
            aws,
            &format!("{prefix}/algorithm.rs"),
            &files.algorithm_file(),
        )
        .await?;
        let metadata = files.metadata_file();
        if metadata.exists() {
            self.upload(aws, &format!("{prefix}/metadata.rs"), &metadata)
                .await?;
        }
        self.upload(aws, &format!("{prefix}/binary.cwasm"), &files.cwasm_file())
            .await
    }

    async fn upload(&self, aws: &Aws, key: &str, path: &Path) -> ACResult<()> {
        aws.s3
            .upload(
                &S3ObjectRef {
                    bucket: S3_BUCKET_UPLOADS.into(),
                    key: key.into(),
                },
                path,
            )
            .await
    }
}

#[async_trait]
impl TaskTrait for CommitTask {
    async fn handle(&self, aws: &Aws) -> ACResult<()> {
        let refs = RepoRefs::new(&self.repo);
        let repo = GitRepo::new(
            &self.repo.account,
            &self.repo.repo,
            Some(&self.branch_name),
            Some(&self.commit_hash),
        );
        let files = Files::new();
        locked(aws, &refs.lock_ref, sync_repo(aws, &files, &refs, &repo)).await?;
        files.copy_algorithms(&self.commit_hash)?;
        files.compile()?;
        self.upload_artifacts(aws, &files).await
    }
}
