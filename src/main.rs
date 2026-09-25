use crate::{
    aws::Aws,
    dto::sqs_event::{SqsEvent, SqsMessage},
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

    async fn handle(&self, event: LambdaEvent<SqsEvent>) -> Result<(), Error> {
        for record in event.payload.records {
            let message: SqsMessage = serde_json::from_str(&record.body)?;
            let task: Task = message.into();
            task.handle(&self.aws).await?;
        }
        Ok(())
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
