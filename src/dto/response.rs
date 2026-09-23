use crate::dto::compile_message::CompileMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileResult {
    pub result: String,
    pub errors: Vec<String>,
    pub compile_messages: Vec<CompileMessage>,
}
