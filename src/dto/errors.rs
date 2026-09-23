use std::fmt::Debug;

#[derive(Debug, thiserror::Error)]
pub enum StockTrekCompileAlgorithmError {
    #[error("InternalServerError: {0:?}")]
    InternalServer(String),
}

pub fn internal_server(message: &str) -> StockTrekCompileAlgorithmError {
    StockTrekCompileAlgorithmError::InternalServer(message.to_string())
}

pub fn internal_server_e<E: Into<Box<dyn std::error::Error>> + Debug>(
    message: &str,
    error: E,
) -> StockTrekCompileAlgorithmError {
    let error_message = format!("{} {:?}", message, error);
    StockTrekCompileAlgorithmError::InternalServer(error_message)
}
