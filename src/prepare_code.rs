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
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::info;

pub fn prepare_code(request: &CompileRequest) -> Result<(), StockTrekCompileAlgorithmError> {
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
    let parent = Path::new(TMP_BUILD_DIR)
        .parent()
        .ok_or_else(|| internal_server_e("Failed to get parent directory of {}", TMP_BUILD_DIR))?;
    fs::create_dir_all(parent)
        .map_err(|e| internal_server_e("Failed to create directory {}", e))?;
    copy_dir_all(ALGORITHM_SOURCE_DIR, TMP_BUILD_DIR)
        .map_err(|e| internal_server_e("Failed to copy directory {}", e))?;
    Ok(())
}

fn copy_dir_all(src: &str, dst: &str) -> Result<(), StockTrekCompileAlgorithmError> {
    let src_path = Path::new(src);
    let dst_path = Path::new(dst);
    if src_path.is_file() {
        fs::copy(src_path, dst_path).map_err(|e| internal_server_e("Failed to copy file {}", e))?;
    } else {
        if !dst_path.exists() {
            fs::create_dir(dst_path)
                .map_err(|e| internal_server_e("Failed to create directory {}", e))?;
        }
        let entries = fs::read_dir(src_path)
            .map_err(|e| internal_server_e("Failed to read directory {}", e))?;
        for entry in entries {
            let entry = entry.map_err(|e| internal_server_e("Failed to read entry {}", e))?;
            let file_type = entry
                .file_type()
                .map_err(|e| internal_server_e("Failed to get file type {}", e))?;
            let file_name = entry.file_name();
            let src_file = entry.path();
            let dst_file = dst_path.join(file_name);
            if file_type.is_file() {
                fs::copy(&src_file, &dst_file)
                    .map_err(|e| internal_server_e("Faild to copy file {}", e))?;
            } else {
                copy_dir_all(&src_file.to_string_lossy(), &dst_file.to_string_lossy())
                    .map_err(|e| internal_server_e("Failed to copy directory {}", e))?;
            }
        }
    }
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
