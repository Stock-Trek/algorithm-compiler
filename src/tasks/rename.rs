use crate::{
    aws::Aws,
    dto::sqs_event::SqsRepoDetail,
    error::ACResult,
    files::Files,
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
}

#[async_trait]
impl TaskTrait for RenameTask {
    async fn handle(&self, aws: &Aws) -> ACResult<()> {
        let from = RepoRefs::new(&self.from);
        let to = RepoRefs::new(&self.to);
        aws.dynamodb
            .locked(&from.lock_ref, async {
                let files = Files::new();
                files.clean()?;
                if !aws.s3.download(&from.repo_ref, &files.archive).await? {
                    return Ok(());
                }
                aws.s3.upload(&to.repo_ref, &files.archive).await?;
                aws.s3
                    .delete_objects_with_prefix(
                        &from.repo_ref.bucket,
                        &RepoRefs::prefix(&self.from),
                    )
                    .await
            })
            .await
    }
}
