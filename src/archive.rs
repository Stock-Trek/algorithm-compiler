use crate::dto::errors::{StockTrekCompileAlgorithmError, internal_server, internal_server_e};
use std::{fs, path::Path, process::Command};

pub fn extract_archive(archive: &Path, dir: &Path) -> Result<(), StockTrekCompileAlgorithmError> {
    run_tar(&["xzf", path_str(archive)?, "-C", path_str(dir)?])
}

pub fn create_archive(dir: &Path, archive: &Path) -> Result<(), StockTrekCompileAlgorithmError> {
    if let Some(parent) = archive.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| internal_server_e("Failed to create archive directory {}", e))?;
    }
    run_tar(&["czf", path_str(archive)?, "-C", path_str(dir)?, "."])
}

fn run_tar(args: &[&str]) -> Result<(), StockTrekCompileAlgorithmError> {
    let output = Command::new("tar")
        .args(args)
        .output()
        .map_err(|e| internal_server_e("Failed to run tar {}", e))?;
    if !output.status.success() {
        return Err(internal_server(&format!(
            "tar {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

fn path_str(path: &Path) -> Result<&str, StockTrekCompileAlgorithmError> {
    path.to_str()
        .ok_or_else(|| internal_server("Path is not valid utf-8"))
}
