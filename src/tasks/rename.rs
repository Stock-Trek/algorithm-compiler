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
    ids: SqsRepoDetail,
    names: SqsRepoDetail,
}

impl RenameTask {
    pub fn new(ids: SqsRepoDetail, names: SqsRepoDetail) -> Self {
        Self { ids, names }
    }
}

#[async_trait]
impl TaskTrait for RenameTask {
    async fn handle(&self, aws: &Aws) -> ACResult<()> {
        let refs = RepoRefs::new(&aws.config, &self.ids);
        let repo = GitRepo::new(&self.names.account, &self.names.repo, None);
        aws.dynamodb
            .locked(&refs.lock_ref, || async {
                let files = Files::new(&aws.config);
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
