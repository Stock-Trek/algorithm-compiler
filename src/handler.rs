use crate::{
    aws::Aws,
    dto::sqs_event::{SqsEvent, SqsEventResponse, SqsMessage},
    error::{ACError, ACResult},
    tasks::task::{Task, TaskTrait},
};
use lambda_runtime::{Error, LambdaEvent};
use std::{sync::Arc, time::SystemTime};
use tracing;

#[derive(Clone)]
pub struct Handler {
    aws: Arc<Aws>,
}

impl Handler {
    pub fn new(aws: Aws) -> Self {
        Self { aws: Arc::new(aws) }
    }

    pub async fn handle(&self, event: LambdaEvent<SqsEvent>) -> Result<SqsEventResponse, Error> {
        let deadline = event.context.deadline();
        let mut response = SqsEventResponse::default();
        for record in event.payload.records {
            if let Err(error) = self.process_record(&record.body, deadline).await {
                if error.is_retryable() {
                    tracing::error!(
                        message_id = %record.message_id,
                        %error,
                        "Failed to process SQS message, reporting batch item failure",
                    );
                    response.add_failure(record.message_id);
                } else {
                    tracing::error!(
                        message_id = %record.message_id,
                        %error,
                        "Failed to process SQS message, discarding it",
                    );
                }
            }
        }
        Ok(response)
    }

    pub async fn process_record(&self, body: &str, deadline: SystemTime) -> ACResult<()> {
        let message: SqsMessage = serde_json::from_str(body).map_err(|error| {
            ACError::InvalidMessage(format!("Failed to deserialize SQS message: {error}"))
        })?;
        let task: Task = message.try_into()?;
        task.handle(&self.aws, deadline).await
    }
}
