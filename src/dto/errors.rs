use std::fmt::Debug;

#[derive(Debug, thiserror::Error)]
pub enum StockTrekCompileAlgorithmError {
    #[error("InvalidRequestError: {0:?}")]
    InvalidRequest(Vec<String>),
    #[error("InternalServerError: {0:?}")]
    InternalServer(String),
}

pub fn invalid_request(errors: &[&str]) -> StockTrekCompileAlgorithmError {
    let vec = errors
        .iter()
        .map(|&s| String::from(s))
        .collect::<Vec<String>>();
    StockTrekCompileAlgorithmError::InvalidRequest(vec)
}

pub fn internal_server(message: &str) -> StockTrekCompileAlgorithmError {
    StockTrekCompileAlgorithmError::InternalServer(message.to_string())
}

pub fn internal_server_e<E: Into<Box<dyn std::error::Error>> + Debug>(
    message: &str,
    error: E,
) -> StockTrekCompileAlgorithmError {
    let error_message = format!("{:?}", error);
    let m = message.replace("{}", error_message.as_str());
    StockTrekCompileAlgorithmError::InternalServer(m)
}
