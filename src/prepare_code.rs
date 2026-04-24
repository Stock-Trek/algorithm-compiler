use crate::{
    constants::*,
    dto::{
        errors::{StockTrekCompileAlgorithmError, internal_server_e},
        request::{CompileRequest, MetadataRequest},
    },
};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::info;

pub fn prepare_code(request: &CompileRequest) -> Result<(), StockTrekCompileAlgorithmError> {
    info!("Prepare code");
    copy_source_folder_to_tmp()?;
    write_code(&request.code)?;
    if let Some(metadata) = &request.metadata {
        write_metadata(metadata)?;
    }
    Ok(())
}

fn copy_source_folder_to_tmp() -> Result<(), StockTrekCompileAlgorithmError> {
    info!("Copy folder to tmp");
    if Path::new(TMP_BUILD_DIR).exists() {
        return Ok(());
    }
    copy_dir(ALGORITHM_SOURCE_DIR, TMP_BUILD_DIR)?;
    Ok(())
}

fn copy_dir(src: &str, dst: &str) -> Result<(), StockTrekCompileAlgorithmError> {
    info!("Copy {} to {}", src, dst);
    list_dir(src)?;
    let _ = Command::new("cp")
        .args(["-r", src, dst])
        .output()
        .map_err(|e| internal_server_e("Error when copying dir {}", e))?;
    list_dir(dst)?;
    Ok(())
}

fn list_dir(dir: &str) -> Result<(), StockTrekCompileAlgorithmError> {
    let output = Command::new("ls")
        .args(["-lA", dir])
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| internal_server_e("Error when listing src dir {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("{}", stdout);
    Ok(())
}

fn write_code(code: &str) -> Result<(), StockTrekCompileAlgorithmError> {
    info!("Copying code");
    let _ = fs::remove_file(TMP_ALGORITHM_RS);
    fs::write(TMP_ALGORITHM_RS, code)
        .map_err(|e| internal_server_e("Failed to write code: {}", e))?;
    Ok(())
}

fn write_metadata(request: &MetadataRequest) -> Result<(), StockTrekCompileAlgorithmError> {
    info!("Write metadata");
    let _ = fs::remove_file(TMP_METADATA_RS);
    let timestamp_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let references_json = serde_json::to_string(&request.metadata.provenance.references)
        .map_err(|e| internal_server_e("Failed to serialize references {}", e))?;
    let metadata_lines = format!(
        r#"
pub const GENERATOR_CREATOR: &str = "{}";
pub const GENERATOR_CREATION_SECONDS_SINCE_EPOCH: i64 = {};
pub const GENERATOR_GENERATOR_ID: &str = "{}";
pub const GENERATOR_NAME: &str = "{}";
pub const GENERATOR_VERSION: &str = "{}";

pub const PROVENANCE_DESCRIPTION: &str = "{}";
pub const PROVENANCE_METHODOLOGY: &str = "{}";
pub const PROVENANCE_REFERENCES: &[&str] = &{};
"#,
        request.metadata.generator.creator,
        timestamp_seconds,
        request.generator_id,
        request.metadata.generator.name,
        request.metadata.generator.version,
        request.metadata.provenance.description,
        request.metadata.provenance.methodology,
        references_json
    );
    info!("Copying metadata");
    info!("{}", metadata_lines);
    fs::write(TMP_METADATA_RS, metadata_lines)
        .map_err(|e| internal_server_e("Failed to write metadata {}", e))?;
    Ok(())
}
