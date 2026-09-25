use serde::{Deserialize, Serialize};

pub const RESULT_SUCCESS: &str = "SUCCESS";
pub const RESULT_FAILURE: &str = "FAILURE";

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileResult {
    pub result: String,
    pub errors: Vec<String>,
    pub compile_messages: Vec<CompileMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileMessage {
    pub level: String,
    pub start: CodeLocation,
    pub end: CodeLocation,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeLocation {
    pub line: i32,
    pub column: i32,
}

impl CompileResult {
    pub fn failed(&self) -> bool {
        self.result == RESULT_FAILURE
    }
}
