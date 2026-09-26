use crate::{
    archive::Archive,
    aws::Aws,
    dto::sqs_event::{SqsRepoId, SqsRepoName},
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    s3::DownloadOutcome,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct RenameRepoTask {
    id: SqsRepoId,
    name: SqsRepoName,
}

impl RenameRepoTask {
    pub fn new(id: SqsRepoId, name: SqsRepoName) -> Self {
        Self { id, name }
    }
}

#[async_trait]
impl TaskTrait for RenameRepoTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let refs = RepoRefs::new(&aws.config, &self.id)?;
        let repo = GitRepo::new(&self.id.clone_url, None, aws.config.timeouts);
        let refs_ref = &refs;
        aws.dynamodb
            .locked(&refs.lock_ref, deadline, move |lock| async move {
                let files = Files::new();
                files.clean()?;
                if aws.s3.download(&refs_ref.repo_ref, &files.archive).await?
                    == DownloadOutcome::NotFound
                {
                    return Ok(());
                }
                Archive::extract(&files.archive, &files.repo)?;
                repo.set_remote(&files.repo, deadline).await?;
                Archive::create(&files.repo, &files.archive)?;
                aws.fenced_s3(&refs_ref.lock_ref, &lock)
                    .upload(&refs_ref.repo_ref, &files.archive)
                    .await
            })
            .await
    }
}
