use crate::{
    aws::Aws,
    dto::{compile_result::CompileMessage, compile_result::CompileResult},
    error::{ACError, ACResult},
    program::Program,
};
use std::{path::PathBuf, process::Command};
use tracing::info;

pub struct Files {
    pub archive: PathBuf,
    pub source: PathBuf,
    pub base: PathBuf,
    pub repo: PathBuf,
    pub build: PathBuf,
    pub algorithms: PathBuf,
}

const FOLDER_BASE: &str = "/tmp/base";
const FILE_ALGORITHMS: &str = "src/algorithms";
const FILE_ARCHIVE: &str = "archive.tar.gz";
const FILE_BUILD: &str = "build";
const FILE_REPO: &str = "repo";
const FILE_SOURCE: &str = "source";
const FILE_BUILT_WASM: &str = "/tmp/base/target/wasm32-wasip1/release/algorithm_runner.wasm";
const FILE_BUILT_CWASM: &str = "/tmp/base/algorithm-runner.cwasm";
const RESULT_SUCCESS: &str = "SUCCESS";
const RESULT_FAILURE: &str = "FAILURE";

impl Files {
    pub fn new() -> Self {
        let base = PathBuf::from(FOLDER_BASE);
        let algorithms = base.join(FILE_ALGORITHMS);
        let archive = base.join(FILE_ARCHIVE);
        let build = base.join(FILE_BUILD);
        let repo = base.join(FILE_REPO);
        let source = base.join(FILE_SOURCE);
        Self {
            algorithms,
            archive,
            base,
            build,
            repo,
            source,
        }
    }
    pub fn clean(&self) -> ACResult<()> {
        std::fs::remove_dir_all(&self.base).map_err(ACError::FileSystem)?;
        std::fs::create_dir_all(&self.base).map_err(ACError::FileSystem)?;
        std::fs::copy(&self.source, &self.build).map_err(ACError::FileSystem)?;
        Ok(())
    }
    pub fn create_build_dir(&self) -> ACResult<()> {
        std::fs::remove_dir_all(&self.base).map_err(ACError::FileSystem)?;
        std::fs::create_dir_all(&self.base).map_err(ACError::FileSystem)?;
        std::fs::copy(&self.source, &self.build).map_err(ACError::FileSystem)?;
        Ok(())
    }
    pub fn sanitize_path(value: &str) -> String {
        value
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' => '_',
                other => other,
            })
            .collect()
    }
    pub fn path_str(path: &PathBuf) -> ACResult<&str> {
        path.to_str()
            .ok_or_else(|| ACError::InternalServer(format!("Path {:?} is not valid utf-8", path)))
    }
}

// copy algorithms

impl Files {
    pub async fn copy_algorithms(&self) -> ACResult<()> {
        let repo_str = Self::path_str(&self.repo)?;
        let algorithms_str = Self::path_str(&self.algorithms)?;
        let mut git_folder_contents = Command::new("git");
        git_folder_contents.args(&[
            &format!("--git-dir={repo_str}"),
            "archive",
            "--format=tar",
            &format!("HEAD:{algorithms_str}"),
        ]);
        let algorithms_folder_str = Self::path_str(&self.algorithms)?;
        let mut copy_contents = Command::new("tar");
        copy_contents.args(&["-x", "-C", algorithms_folder_str]);
        let mut commands = [git_folder_contents, copy_contents];
        Program::pipe(&mut commands)?;
        Ok(())
    }
}

// compile
impl Files {
    pub fn compile(&self) -> ACResult<CompileResult> {
        info!("compile");
        let build_result = self.build_wasm()?;
        info!("Build result {:?}", build_result);
        if build_result.result == RESULT_FAILURE {
            Ok(build_result)
        } else {
            let compile_result = self.compile_cwasm()?;
            info!("Build result {:?}", compile_result);
            Ok(compile_result)
        }
    }

    fn build_wasm(&self) -> ACResult<CompileResult> {
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
            .map_err(|e| internal_server_e("Error when calling build process", e))?;
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

    fn compile_cwasm(&self) -> ACResult<CompileResult> {
        info!("Compiling cwasm");
        std::fs::remove_file(FILE_BUILT_CWASM).map_err(ACError::FileSystem)?;
        Program::run(
            "wasmtime",
            &[
                "compile",
                "-C",
                "cache=no",
                FILE_BUILT_WASM,
                "-o",
                FILE_BUILT_CWASM,
            ],
            &self.build,
        )?;
        Ok(CompileResult {
            result: RESULT_SUCCESS.to_string(),
            errors: vec![],
            compile_messages: vec![],
        })
    }

    fn get_compile_output(stdout: String) -> ACResult<CompileOutput> {
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
                .map_err(|e| internal_server_e("Failed to parse values from json", e))?;
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
    ) -> ACResult<()> {
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
}
