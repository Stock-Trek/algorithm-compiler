use serde::{Deserialize, Serialize};

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
