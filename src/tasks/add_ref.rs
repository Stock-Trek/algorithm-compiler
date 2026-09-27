use crate::{
    archive::Archive,
    aws::Aws,
    dto::sqs_event::{SqsRefType, SqsRepoId},
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    s3::DownloadOutcome,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddRefTask {
    id: SqsRepoId,
    ref_name: String,
    ref_type: SqsRefType,
}

impl AddRefTask {
    pub fn new(id: SqsRepoId, ref_name: String, ref_type: SqsRefType) -> Self {
        Self {
            id,
            ref_name,
            ref_type,
        }
    }
}

#[async_trait]
impl TaskTrait for AddRefTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let refs = RepoRefs::new(&aws.config, &self.id)?;
        let repo = GitRepo::new(aws.config.timeouts);
        let refs_ref = &refs;
        let ref_name = self.ref_name.as_str();
        let ref_type = self.ref_type;
        let files = Files::new();
        aws.dynamodb
            .locked(&refs.lock_ref, deadline, move |lock| async move {
                files.clean()?;
                if aws.s3.download(&refs_ref.repo_ref, &files.archive).await?
                    == DownloadOutcome::NotFound
                {
                    return Ok(());
                }
                Archive::extract(&files.archive, &files.repo)?;
                repo.add_ref(&files.repo, ref_name, ref_type, deadline)
                    .await?;
                Archive::create(&files.repo, &files.archive)?;
                aws.fenced_s3(&refs_ref.lock_ref, &lock)
                    .upload(&refs_ref.repo_ref, &files.archive)
                    .await
            })
            .await
    }
}
