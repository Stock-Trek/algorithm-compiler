use crate::{
    dto::{
        errors::{StockTrekCompileAlgorithmError, internal_server_e},
        response::to_http_response,
    },
    handle_event::handle_event,
};
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use serde_json::{Value, json};
use std::process::{Command, Stdio};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt::Subscriber};

mod compile;
mod constants;
mod dto;
mod handle_event;
mod prepare_code;
mod s3;
mod upload;

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_tracing()?;
    pwd()?;
    list_dir(".")?;
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

fn pwd() -> Result<(), StockTrekCompileAlgorithmError> {
    let output = Command::new("pwd")
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| internal_server_e("Error when listing src dir {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("{}", stdout);
    Ok(())
}

fn list_dir(dir: &str) -> Result<(), StockTrekCompileAlgorithmError> {
    let output = Command::new("ls")
        .args(["-lA", dir])
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| internal_server_e("Error when listing src dir {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("{}", stdout);
    Ok(())
}

async fn function_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let http_response = match handle_event(event).await {
        Ok(response) => response,
        Err(error) => to_http_response(error),
    };
    Ok(json!(http_response))
}
