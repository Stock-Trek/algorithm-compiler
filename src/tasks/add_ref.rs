use crate::{
    archive::Archive,
    aws::Aws,
    dto::sqs_event::{GitSource, SqsRefType},
    error::ACResult,
    files::Files,
    s3::DownloadOutcome,
    tasks::{repo_refs::RepoRefs, task::TaskTrait},
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct AddRefTask {
    source: GitSource,
    repo_id: String,
    ref_name: String,
    ref_type: SqsRefType,
}

impl AddRefTask {
    pub fn new(source: GitSource, repo_id: String, ref_name: String, ref_type: SqsRefType) -> Self {
        Self {
            source,
            repo_id,
            ref_name,
            ref_type,
        }
    }
}

#[async_trait]
impl TaskTrait for AddRefTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        let refs = RepoRefs::new(&aws.config, self.source.clone(), &self.repo_id)?;
        let source = &self.source;
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
                source
                    .add_ref(aws, &files.repo, ref_name, ref_type, deadline)
                    .await?;
                Archive::create(&files.repo, &files.archive)?;
                aws.fenced_s3(&refs_ref.lock_ref, &lock)
                    .upload(&refs_ref.repo_ref, &files.archive)
                    .await
            })
            .await
    }
}
