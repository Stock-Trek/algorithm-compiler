use crate::{
    archive::Archive,
    aws::Aws,
    dto::sqs_event::SqsRepoDetail,
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
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
        let refs = RepoRefs::new(&self.from);
        let repo = GitRepo::new(&self.to.account, &self.to.repo, None, None);
        aws.dynamodb
            .locked(&refs.lock_ref, async {
                let files = Files::new();
                files.clean()?;
                if !aws.s3.download(&refs.repo_ref, &files.archive).await? {
                    return Ok(());
                }
                Archive::extract(&files.archive, &files.repo)?;
                repo.set_remote(&files.repo)?;
                Archive::create(&files.repo, &files.archive)?;
                aws.s3.upload(&refs.repo_ref, &files.archive).await
            })
            .await
    }
}
