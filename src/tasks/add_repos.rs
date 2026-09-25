use crate::{
    aws::Aws,
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;

pub struct AddReposTask {
    repo_refs: Vec<RepoRefs>,
    files: Files,
    repo: GitRepo,
}

#[async_trait]
impl TaskTrait for AddReposTask {
    async fn handle(&self, aws: &Aws) -> ACResult<()> {}
}
