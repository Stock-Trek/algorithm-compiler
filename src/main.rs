use crate::{
    aws::Aws,
    dto::sqs_event::{SqsEvent, SqsMessage},
    task::TaskOld,
};
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use std::sync::Arc;
use tracing_subscriber::{EnvFilter, fmt::Subscriber};

mod archive;
mod aws;
mod dto;
mod dynamodb;
mod error;
mod files;
mod git_repo;
mod program;
mod s3;
mod task;
mod tasks;

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_tracing()?;
    let aws = Arc::new(Aws::new().await);
    let handler = move |event: LambdaEvent<SqsEvent>| {
        let aws_clone = aws.clone();
        async move { function_handler(&aws_clone, event).await }
    };
    run(service_fn(handler)).await
}

fn setup_tracing() -> Result<(), Error> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = Subscriber::builder()
        .with_ansi(false)
        .with_env_filter(filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
    Ok(())
}

async fn function_handler(aws: &Aws, event: LambdaEvent<SqsEvent>) -> Result<(), Error> {
    for record in event.payload.records {
        let message: SqsMessage = serde_json::from_str(&record.body)?;
        let task: TaskOld = message.into();
        task.handle(aws).await?;
    }
    Ok(())
}
