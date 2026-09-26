use crate::{
    archive::Archive,
    aws::Aws,
    dto::sqs_event::{GitProvider, SqsRepoDetail},
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct RenameRepoTask {
    provider: GitProvider,
    ids: SqsRepoDetail,
    names: SqsRepoDetail,
}

impl RenameRepoTask {
    pub fn new(provider: GitProvider, ids: SqsRepoDetail, names: SqsRepoDetail) -> Self {
        Self {
            provider,
            ids,
            names,
        }
    }
}

#[async_trait]
impl TaskTrait for RenameRepoTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let refs = RepoRefs::new(&aws.config, &self.ids)?;
        let repo = GitRepo::new(&self.provider, &self.names.account, &self.names.repo, None);
        aws.dynamodb
            .locked(&refs.lock_ref, deadline, || async {
                let files = Files::new();
                files.clean()?;
                if !aws.s3.download(&refs.repo_ref, &files.archive).await? {
                    return Ok(());
                }
                Archive::extract(&files.archive, &files.repo)?;
                repo.set_remote(&files.repo).await?;
                Archive::create(&files.repo, &files.archive)?;
                aws.s3.upload(&refs.repo_ref, &files.archive).await
            })
            .await
    }
}
