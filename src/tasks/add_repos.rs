use crate::{
    aws::Aws,
    dto::sqs_event::{GitProvider, SqsRepoId},
    error::ACResult,
    files::Files,
    git_repo::GitRepo,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddReposTask {
    provider: GitProvider,
    ids: Vec<SqsRepoId>,
}

impl AddReposTask {
    pub fn new(provider: GitProvider, ids: Vec<SqsRepoId>) -> Self {
        Self { provider, ids }
    }
}

#[async_trait]
impl TaskTrait for AddReposTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let repo = GitRepo::new(aws.config.timeouts);
        for id in &self.ids {
            let clone_url = "TODO";
            let ref_name = "TODO";
            let commit_hash = "TODO";
            let refs = RepoRefs::new(&aws.config, id)?;
            let files = Files::new();
            let refs_ref = &refs;
            let repo_ref = &repo;
            aws.dynamodb
                .locked(&refs.lock_ref, deadline, move |lock| async move {
                    refs_ref
                        .sync(
                            clone_url,
                            ref_name,
                            commit_hash,
                            aws,
                            files,
                            repo_ref,
                            &lock,
                            deadline,
                        )
                        .await
                })
                .await?;
        }
        Ok(())
    }
}
