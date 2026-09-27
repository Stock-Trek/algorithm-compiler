use crate::{aws::Aws, dto::sqs_event::GitProvider, error::ACResult, github::GitHubRepo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub id: u64,
    pub name: String,
    pub owner: String,
    pub clone_url: String,
}

impl From<GitHubRepo> for Repo {
    fn from(repo: GitHubRepo) -> Self {
        Self {
            id: repo.id,
            name: repo.name,
            owner: repo.owner.login,
            clone_url: repo.clone_url,
        }
    }
}

impl GitProvider {
    pub async fn repos(&self, aws: &Aws) -> ACResult<Vec<Repo>> {
        match self {
            Self::GitHub {
                installation_id, ..
            } => Ok(aws
                .github
                .repos(*installation_id)
                .await?
                .into_iter()
                .map(Repo::from)
                .collect()),
        }
    }

    pub async fn repo(&self, aws: &Aws, repo_id: u64) -> ACResult<Repo> {
        match self {
            Self::GitHub {
                installation_id, ..
            } => Ok(aws.github.repo(*installation_id, repo_id).await?.into()),
        }
    }
}
