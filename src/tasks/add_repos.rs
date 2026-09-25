use crate::{
    aws::Aws,
    dto::sqs_event::SqsRepoDetail,
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    tasks::{
        repo_refs::{RepoRefs, locked, sync_repo},
        task::TaskTrait,
    },
};
use async_trait::async_trait;

pub struct AddReposTask {
    repos: Vec<SqsRepoDetail>,
}

impl AddReposTask {
    pub fn new(repos: Vec<SqsRepoDetail>) -> Self {
        Self { repos }
    }
}

#[async_trait]
impl TaskTrait for AddReposTask {
    async fn handle(&self, aws: &Aws) -> ACResult<()> {
        for detail in &self.repos {
            let refs = RepoRefs::new(detail);
            let repo = GitRepo::new(&detail.account, &detail.repo, None, None);
            let files = Files::new();
            locked(aws, &refs.lock_ref, sync_repo(aws, &files, &refs, &repo)).await?;
        }
        Ok(())
    }
}
