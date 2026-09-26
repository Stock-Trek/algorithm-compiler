use crate::{
    aws::Aws,
    dto::sqs_event::{SqsRefType, SqsRepoId},
    error::ACResult,
    tasks::task::TaskTrait,
};
use async_trait::async_trait;
use std::time::SystemTime;

pub struct DeleteRefTask {
    id: SqsRepoId,
    ref_name: String,
    ref_type: SqsRefType,
}

impl DeleteRefTask {
    pub fn new(id: SqsRepoId, ref_name: String, ref_type: SqsRefType) -> Self {
        Self {
            id,
            ref_name,
            ref_type,
        }
    }
}

#[async_trait]
impl TaskTrait for DeleteRefTask {
    async fn handle(&self, aws: &Aws, deadline: SystemTime) -> ACResult<()> {
        Ok(())
    }
}
