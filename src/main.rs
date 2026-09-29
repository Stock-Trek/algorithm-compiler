use crate::{aws::Aws, config::Config, handler::Handler, tracing_setup::TracingSetup};
use lambda_runtime::{Error, run, service_fn};

mod archive;
mod aws;
mod config;
mod constants;
mod dto;
mod dynamodb;
mod error;
mod fenced;
mod files;
mod git_local;
mod git_remote;
mod github;
mod handler;
mod program;
mod s3;
mod sqs;
mod tasks;
mod timeouts;
mod tracing_setup;

#[tokio::main]
async fn main() -> Result<(), Error> {
    TracingSetup::setup()?;
    let config = Config::from_env()?;
    let handler = Handler::new(Aws::new(config).await?);
    run(service_fn(move |event| {
        let handler = handler.clone();
        async move { handler.handle(event).await }
    }))
    .await
}
