use std::time::Duration;

const COMMAND_TIMEOUT_ENV: &str = "COMMAND_TIMEOUT_SECONDS";
const AWS_CONNECT_TIMEOUT_ENV: &str = "AWS_CONNECT_TIMEOUT_SECONDS";
const AWS_OPERATION_TIMEOUT_ENV: &str = "AWS_OPERATION_TIMEOUT_SECONDS";

const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(600);
const DEFAULT_AWS_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_AWS_OPERATION_TIMEOUT: Duration = Duration::from_secs(120);

pub fn command() -> Duration {
    duration_from_env(COMMAND_TIMEOUT_ENV, DEFAULT_COMMAND_TIMEOUT)
}

pub fn aws_connect() -> Duration {
    duration_from_env(AWS_CONNECT_TIMEOUT_ENV, DEFAULT_AWS_CONNECT_TIMEOUT)
}

pub fn aws_operation() -> Duration {
    duration_from_env(AWS_OPERATION_TIMEOUT_ENV, DEFAULT_AWS_OPERATION_TIMEOUT)
}

fn duration_from_env(key: &str, default: Duration) -> Duration {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map(Duration::from_secs)
        .unwrap_or(default)
}
