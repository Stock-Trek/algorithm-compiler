use crate::{
    constants::RESULT_FAILURE,
    dto::{
        compile_message::CompileMessage,
        errors::StockTrekCompileAlgorithmError::{self, InternalServer, InvalidRequest},
    },
};
use serde::{Deserialize, Serialize};
use tracing::error;

pub fn to_http_response(error: StockTrekCompileAlgorithmError) -> HttpResponse {
    match error {
        InvalidRequest(missing_data) => HttpResponse {
            status_code: 400,
            body: HandlerResult {
                compile_result: CompileResult {
                    result: RESULT_FAILURE.to_string(),
                    errors: missing_data.clone(),
                    compile_messages: vec![],
                },
                saved_generator_id: None,
            },
        },
        InternalServer(message) => {
            error!("InternalServerError: {}", message);
            HttpResponse {
                status_code: 500,
                body: HandlerResult {
                    compile_result: CompileResult {
                        result: RESULT_FAILURE.to_string(),
                        errors: vec!["InternalServerError".to_string()],
                        compile_messages: vec![],
                    },
                    saved_generator_id: None,
                },
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    #[serde(rename = "statusCode")]
    pub status_code: i32,
    pub body: HandlerResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HandlerResult {
    #[serde(flatten)]
    pub compile_result: CompileResult,
    pub saved_generator_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileResult {
    pub result: String,
    pub errors: Vec<String>,
    pub compile_messages: Vec<CompileMessage>,
}
