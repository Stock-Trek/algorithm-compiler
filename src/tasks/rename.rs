use crate::{
    aws::Aws,
    dto::sqs_event::SqsRepoDetail,
    error::{ACError, ACResult},
    files::Files,
    s3::S3ObjectRef,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;

pub struct RenameTask {
    from: SqsRepoDetail,
    to: SqsRepoDetail,
}

impl RenameTask {
    pub fn new(from: SqsRepoDetail, to: SqsRepoDetail) -> Self {
        Self { from, to }
    }

    async fn rename(&self, aws: &Aws) -> ACResult<()> {
        let from = RepoRefs::new(&self.from);
        let to = RepoRefs::new(&self.to);
        let from_prefix = RepoRefs::prefix(&self.from);
        let to_prefix = RepoRefs::prefix(&self.to);

        let files = Files::new();
        files.clean()?;
        if !aws.s3.download(&from.repo_ref, &files.archive).await? {
            return Ok(());
        }

        // Never overwrite an existing destination repository.
        let existing = aws
            .s3
            .list_keys_with_prefix(&to.repo_ref.bucket, &to_prefix)
            .await?;
        if !existing.is_empty() {
            return Err(ACError::InternalServer(format!(
                "Cannot rename to {to_prefix}: destination already exists"
            )));
        }

        // Move the compiled artifacts to the destination, preserving their
        // commit-hash sub-prefixes, so they are not destroyed by the rename.
        let source_keys = aws
            .s3
            .list_keys_with_prefix(&from.repo_ref.bucket, &from_prefix)
            .await?;
        for key in &source_keys {
            if key == &from.repo_ref.key {
                continue;
            }
            let suffix = key.strip_prefix(&from_prefix).ok_or_else(|| {
                ACError::InternalServer(format!("Source key {key} is not under {from_prefix}"))
            })?;
            aws.s3
                .copy(
                    &S3ObjectRef {
                        bucket: from.repo_ref.bucket.clone(),
                        key: key.clone(),
                    },
                    &S3ObjectRef {
                        bucket: to.repo_ref.bucket.clone(),
                        key: format!("{to_prefix}{suffix}"),
                    },
                )
                .await?;
        }

        // The archive is keyed by the repository name, so upload it under the
        // destination key rather than copying it.
        aws.s3.upload(&to.repo_ref, &files.archive).await?;

        // Only remove the source objects once every copy has succeeded.
        aws.s3
            .delete_keys(&from.repo_ref.bucket, &source_keys)
            .await
    }
}

#[async_trait]
impl TaskTrait for RenameTask {
    async fn handle(&self, aws: &Aws) -> ACResult<()> {
        let from = RepoRefs::new(&self.from);
        let to = RepoRefs::new(&self.to);
        aws.dynamodb
            .locked_many(&[&from.lock_ref, &to.lock_ref], self.rename(aws))
            .await
    }
}
