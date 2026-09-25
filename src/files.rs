use crate::{
    dto::compile_result::{CodeLocation, CompileMessage, CompileResult},
    error::{ACError, ACResult},
    program::Program,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tracing::{info, warn};

const BASE: &str = "/tmp/algorithm-compiler";
const SOURCE: &str = "./algorithm-runner";
const REPO_FOLDER: &str = "repo";
const ARCHIVE_FILE: &str = "archive.tar.gz";
const BUILD_FOLDER: &str = "build";
const ALGORITHMS_FOLDER: &str = "src/algorithms";
const CHECKED_FILE_PATH: &str = "src/algorithms/algorithm.rs";
const BUILT_WASM: &str = "target/wasm32-wasip1/release/algorithm_runner.wasm";
const BUILT_CWASM: &str = "algorithm-runner.cwasm";
const COMPILER_MESSAGE: &str = "compiler-message";
const BUILD_FINISHED: &str = "build-finished";
const LEVEL_ERROR: &str = "error";
const RESULT_SUCCESS: &str = "SUCCESS";
const RESULT_FAILURE: &str = "FAILURE";

struct CompileOutput {
    success: bool,
    compile_messages: Vec<CompileMessage>,
}

pub struct Files {
    pub base: PathBuf,
    pub archive: PathBuf,
    pub repo: PathBuf,
    pub build: PathBuf,
    pub algorithms: PathBuf,
}

impl Files {
    pub fn new() -> Self {
        let base = PathBuf::from(BASE);
        let build = base.join(BUILD_FOLDER);
        Self {
            archive: base.join(ARCHIVE_FILE),
            repo: base.join(REPO_FOLDER),
            algorithms: build.join(ALGORITHMS_FOLDER),
            build,
            base,
        }
    }

    pub fn clean(&self) -> ACResult<()> {
        let _ = fs::remove_dir_all(&self.base);
        fs::create_dir_all(&self.repo).map_err(ACError::FileSystem)?;
        fs::create_dir_all(&self.build).map_err(ACError::FileSystem)?;
        Ok(())
    }

    pub fn prepare(&self) -> ACResult<()> {
        self.clean()?;
        copy_dir(Path::new(SOURCE), &self.build)
    }

    pub fn copy_algorithms(&self, revision: &str) -> ACResult<()> {
        fs::create_dir_all(&self.algorithms).map_err(ACError::FileSystem)?;
        let git_dir = Self::path_str(&self.repo)?;
        let algorithms = Self::path_str(&self.algorithms)?;
        let mut git = Command::new("git");
        git.args([
            &format!("--git-dir={git_dir}"),
            "archive",
            "--format=tar",
            &format!("{revision}:{ALGORITHMS_FOLDER}"),
        ]);
        let mut tar = Command::new("tar");
        tar.args(["-x", "-C", algorithms]);
        let mut commands = [git, tar];
        Program::pipe(&mut commands)?;
        Ok(())
    }

    pub fn compile(&self) -> ACResult<CompileResult> {
        info!("compile");
        let build_result = self.build_wasm()?;
        info!("Build result {:?}", build_result);
        if build_result.result == RESULT_FAILURE {
            return Ok(build_result);
        }
        self.compile_cwasm()
    }

    pub fn algorithm_file(&self) -> PathBuf {
        self.algorithms.join("algorithm.rs")
    }

    pub fn metadata_file(&self) -> PathBuf {
        self.algorithms.join("metadata.rs")
    }

    pub fn cwasm_file(&self) -> PathBuf {
        self.build.join(BUILT_CWASM)
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

    pub fn path_str(path: &Path) -> ACResult<&str> {
        path.to_str()
            .ok_or_else(|| ACError::InternalServer(format!("Path {:?} is not valid utf-8", path)))
    }

    fn build_wasm(&self) -> ACResult<CompileResult> {
        info!("Building wasm");
        let _ = fs::remove_file(self.build.join(BUILT_WASM));
        let output = Command::new("cargo")
            .args([
                "build",
                "--frozen",
                "--target=wasm32-wasip1",
                "--release",
                "--message-format=json",
                "--quiet",
            ])
            .current_dir(&self.build)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map_err(ACError::CommandRun)?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let compile_output = get_compile_output(stdout)?;
        let error_count = compile_output
            .compile_messages
            .iter()
            .filter(|message| message.level == LEVEL_ERROR)
            .count();
        if !compile_output.success && error_count == 0 {
            return Err(ACError::InternalServer(
                "Build failed but did not detect compile errors".into(),
            ));
        }
        if compile_output.success && error_count > 0 {
            return Err(ACError::InternalServer(
                "Build succeeded but detected compile errors".into(),
            ));
        }
        if !compile_output.success {
            return Ok(CompileResult {
                result: RESULT_FAILURE.into(),
                errors: vec![],
                compile_messages: compile_output.compile_messages,
            });
        }
        if !self.build.join(BUILT_WASM).exists() {
            return Err(ACError::InternalServer("WASM file was not built".into()));
        }
        Ok(CompileResult {
            result: RESULT_SUCCESS.into(),
            errors: vec![],
            compile_messages: vec![],
        })
    }

    fn compile_cwasm(&self) -> ACResult<CompileResult> {
        info!("Compiling cwasm");
        let _ = fs::remove_file(self.build.join(BUILT_CWASM));
        Program::run(
            "wasmtime",
            &["compile", "-C", "cache=no", BUILT_WASM, "-o", BUILT_CWASM],
            &self.build,
        )?;
        Ok(CompileResult {
            result: RESULT_SUCCESS.into(),
            errors: vec![],
            compile_messages: vec![],
        })
    }
}

fn get_compile_output(stdout: String) -> ACResult<CompileOutput> {
    info!("Get compile output from stdout");
    let mut success = false;
    let mut compile_messages = Vec::new();
    for raw_line in stdout.lines() {
        let cleaned_line = raw_line.trim();
        if cleaned_line.is_empty() {
            continue;
        }
        let values =
            serde_json::from_str::<HashMap<String, Value>>(cleaned_line).map_err(|error| {
                ACError::InternalServer(format!("Failed to parse compile output: {error}"))
            })?;
        let Some(reason) = values.get("reason").and_then(|value| value.as_str()) else {
            continue;
        };
        match reason {
            COMPILER_MESSAGE => add_compiler_messages(&mut compile_messages, &values),
            BUILD_FINISHED => {
                success = values
                    .get("success")
                    .and_then(|value| value.as_bool())
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

fn add_compiler_messages(
    compile_messages: &mut Vec<CompileMessage>,
    values: &HashMap<String, Value>,
) {
    let Some(message_dict) = values.get("message").and_then(|value| value.as_object()) else {
        return;
    };
    let Some(level) = message_dict.get("level").and_then(|value| value.as_str()) else {
        return;
    };
    let Some(message) = message_dict.get("message").and_then(|value| value.as_str()) else {
        return;
    };
    let Some(spans) = message_dict.get("spans").and_then(|value| value.as_array()) else {
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
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if !is_primary {
        return None;
    }
    let file_name = span
        .get("file_name")
        .and_then(|value| value.as_str())
        .unwrap_or("");
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
    CompileMessage {
        start: CodeLocation {
            line: to_int(span, "line_start"),
            column: to_int(span, "column_start"),
        },
        end: CodeLocation {
            line: to_int(span, "line_end"),
            column: to_int(span, "column_end"),
        },
        level,
        message,
    }
}

fn to_int(span: &Value, key: &str) -> i32 {
    span.get(key).and_then(|value| value.as_i64()).unwrap_or(0) as i32
}

fn copy_dir(source: &Path, destination: &Path) -> ACResult<()> {
    fs::create_dir_all(destination).map_err(ACError::FileSystem)?;
    for entry in fs::read_dir(source).map_err(ACError::FileSystem)? {
        let entry = entry.map_err(ACError::FileSystem)?;
        let file_type = entry.file_type().map_err(ACError::FileSystem)?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target).map_err(ACError::FileSystem)?;
        }
    }
    Ok(())
}
