use crate::{
    dto::sqs::{SqsEvent, SqsMessage},
    handle_event::handle_event,
};
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use tracing_subscriber::{EnvFilter, fmt::Subscriber};

mod compile;
mod constants;
mod dto;
mod dynamodb;
mod handle_event;
mod prepare_code;
mod repo;
mod s3;
mod upload;

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_tracing()?;
    run(service_fn(function_handler)).await
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

async fn function_handler(event: LambdaEvent<SqsEvent>) -> Result<(), Error> {
    for record in event.payload.records {
        let message: SqsMessage = serde_json::from_str(&record.body)?;
        handle_event(message).await?;
    }
    Ok(())
}
