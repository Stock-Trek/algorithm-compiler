use crate::{
    constants::{
        BUILD_FINISHED, BUILT_CWASM, BUILT_WASM, CHECKED_FILE_PATH, COMPILER_MESSAGE, LEVEL_ERROR,
        RESULT_FAILURE, RESULT_SUCCESS, TMP_BUILD_DIR,
    },
    dto::{
        compile_message::{CodeLocation, CompileMessage},
        errors::{StockTrekCompileAlgorithmError, internal_server, internal_server_e},
        response::CompileResult,
    },
};
use lambda_runtime::tracing::{info, warn};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::{Command, Stdio},
};

pub fn compile() -> Result<CompileResult, StockTrekCompileAlgorithmError> {
    info!("compile");
    let build_result = build_wasm()?;
    info!("Build result {:?}", build_result);
    if build_result.result == RESULT_FAILURE {
        return Ok(build_result);
    } else {
        let compile_result = compile_cwasm()?;
        info!("Build result {:?}", compile_result);
        return Ok(compile_result);
    }
}

fn build_wasm() -> Result<CompileResult, StockTrekCompileAlgorithmError> {
    info!("Building wasm");
    let _ = fs::remove_file(BUILT_WASM);
    let output = Command::new("cargo")
        .args([
            "build",
            "--frozen",
            "--target=wasm32-wasip1",
            "--release",
            "--message-format=json",
            "--quiet",
        ])
        .current_dir(TMP_BUILD_DIR)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| internal_server_e("Error when calling build process {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let comple_output = get_compile_output(stdout.to_string())?;
    let compile_error_count = comple_output
        .compile_messages
        .iter()
        .filter(|cm| cm.level == LEVEL_ERROR)
        .fold(0, |acc, _| acc + 1);
    if !comple_output.success && compile_error_count == 0 {
        return Err(internal_server(
            "Build failed but did not detect compile errors",
        ));
    }
    if comple_output.success && compile_error_count > 0 {
        return Err(internal_server(
            "Build succeeded but detected compile errors",
        ));
    }
    if !comple_output.success {
        return Ok(CompileResult {
            result: RESULT_FAILURE.to_string(),
            errors: vec![],
            compile_messages: comple_output.compile_messages,
        });
    }
    if !Path::new(BUILT_WASM).exists() {
        return Err(internal_server("WASM file was not built"));
    }
    Ok(CompileResult {
        result: RESULT_SUCCESS.to_string(),
        errors: vec![],
        compile_messages: vec![],
    })
}

fn compile_cwasm() -> Result<CompileResult, StockTrekCompileAlgorithmError> {
    info!("Compiling cwasm");
    let _ = fs::remove_file(BUILT_CWASM);
    let output = Command::new("wasmtime")
        .args(["compile", "-C", "cache=no", BUILT_WASM, "-o", BUILT_CWASM])
        .current_dir(TMP_BUILD_DIR)
        .output()
        .map_err(|e| internal_server_e("Unknown compile error {}", e))?;
    if !output.status.success() {
        info!("{}", String::from_utf8_lossy(&output.stderr));
        return Err(internal_server("Failed to compile cwasm"));
    }
    Ok(CompileResult {
        result: RESULT_SUCCESS.to_string(),
        errors: vec![],
        compile_messages: vec![],
    })
}

struct CompileOutput {
    success: bool,
    compile_messages: Vec<CompileMessage>,
}

fn get_compile_output(stdout: String) -> Result<CompileOutput, StockTrekCompileAlgorithmError> {
    info!("Get compile output from stdout");
    let mut success = false;
    let mut compile_messages = Vec::new();
    let lines = stdout.split('\n');
    for raw_line in lines {
        let cleaned_line = raw_line.trim();
        if cleaned_line.is_empty() {
            continue;
        }
        let values = serde_json::from_str::<HashMap<String, Value>>(cleaned_line)
            .map_err(|e| internal_server_e("Failed to parse values from json {}", e))?;
        let Some(reason) = values.get("reason").and_then(|v: &Value| v.as_str()) else {
            continue;
        };
        match reason {
            COMPILER_MESSAGE => {
                add_compiler_messages_from_values(&mut compile_messages, values);
            }
            BUILD_FINISHED => {
                success = values
                    .get("success")
                    .and_then(|v: &Value| v.as_bool())
                    .unwrap_or(false);
            }
            _ => {}
        }
    }
    Ok(CompileOutput {
        success,
        compile_messages,
    })
}

fn add_compiler_messages_from_values(
    compile_messages: &mut Vec<CompileMessage>,
    values: HashMap<String, Value>,
) -> () {
    let Some(message_dict) = values.get("message").and_then(|v| v.as_object()) else {
        return;
    };
    let Some(level) = message_dict.get("level").and_then(|v| v.as_str()) else {
        return;
    };
    let Some(message) = message_dict.get("message").and_then(|v| v.as_str()) else {
        return;
    };
    let Some(spans) = message_dict.get("spans").and_then(|v| v.as_array()) else {
        return;
    };
    for span in spans {
        if let Some(compile_message) = span_to_compile_message(level, message, span) {
            compile_messages.push(compile_message);
        }
    }
}

fn span_to_compile_message(level: &str, message: &str, span: &Value) -> Option<CompileMessage> {
    let is_primary = span
        .get("is_primary")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !is_primary {
        return None;
    }
    let file_name = span.get("file_name").and_then(|v| v.as_str()).unwrap_or("");
    if file_name != CHECKED_FILE_PATH {
        warn!(
            "Compile message from external file '{}': {} - {}",
            file_name, level, message
        );
        return None;
    }
    Some(to_compile_message(
        level.to_string(),
        message.to_string(),
        span,
    ))
}

fn to_compile_message(level: String, message: String, span: &Value) -> CompileMessage {
    let line_start = to_int(span, "line_start");
    let line_end = to_int(span, "line_end");
    let column_start = to_int(span, "column_start");
    let column_end = to_int(span, "column_end");
    CompileMessage {
        start: CodeLocation {
            line: line_start,
            column: column_start,
        },
        end: CodeLocation {
            line: line_end,
            column: column_end,
        },
        level,
        message,
    }
}

fn to_int(span: &Value, key: &str) -> i32 {
    span.get(key).and_then(|v| v.as_i64()).unwrap_or(0) as i32
}
