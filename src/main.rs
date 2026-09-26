use crate::{
    aws::Aws,
    dto::sqs_event::{SqsEvent, SqsEventResponse, SqsMessage},
    error::{ACError, ACResult},
    tasks::task::{Task, TaskTrait},
};
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use std::sync::Arc;
use tracing_subscriber::{EnvFilter, fmt::Subscriber};

mod archive;
mod aws;
mod constants;
mod dto;
mod dynamodb;
mod error;
mod files;
mod git_repo;
mod program;
mod s3;
mod tasks;

struct Tracing;

impl Tracing {
    fn setup() -> Result<(), Error> {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let subscriber = Subscriber::builder()
            .with_ansi(false)
            .with_env_filter(filter)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
        Ok(())
    }
}

#[derive(Clone)]
struct Handler {
    aws: Arc<Aws>,
}

impl Handler {
    fn new(aws: Aws) -> Self {
        Self { aws: Arc::new(aws) }
    }

    async fn process_record(&self, body: &str) -> ACResult<()> {
        let message: SqsMessage = serde_json::from_str(body).map_err(|error| {
            ACError::InvalidMessage(format!("Failed to deserialize SQS message: {error}"))
        })?;
        let task: Task = message.into();
        task.handle(&self.aws).await
    }

    async fn handle(&self, event: LambdaEvent<SqsEvent>) -> Result<SqsEventResponse, Error> {
        let mut response = SqsEventResponse::default();
        for record in event.payload.records {
            if let Err(error) = self.process_record(&record.body).await {
                tracing::error!(
                    message_id = %record.message_id,
                    %error,
                    "Failed to process SQS message, reporting batch item failure",
                );
                response.add_failure(record.message_id);
            }
        }
        Ok(response)
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    Tracing::setup()?;
    let handler = Handler::new(Aws::new().await);
    run(service_fn(move |event| {
        let handler = handler.clone();
        async move { handler.handle(event).await }
    }))
    .await
}
