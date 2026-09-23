use crate::{
    dto::errors::{StockTrekCompileAlgorithmError, internal_server, internal_server_e},
    program::Program,
};
use std::{fs, path::Path};

const TAR: &str = "tar";

pub fn extract_archive(archive: &Path, dir: &Path) -> Result<(), StockTrekCompileAlgorithmError> {
    Program::run(TAR, &["xzf", path_str(archive)?, "-C", path_str(dir)?], dir)
}

pub fn create_archive(dir: &Path, archive: &Path) -> Result<(), StockTrekCompileAlgorithmError> {
    if let Some(parent) = archive.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| internal_server_e("Failed to create archive directory", e))?;
    }
    Program::run(
        TAR,
        &["czf", path_str(archive)?, "-C", path_str(dir)?, "."],
        dir,
    )
}

fn path_str(path: &Path) -> Result<&str, StockTrekCompileAlgorithmError> {
    path.to_str()
        .ok_or_else(|| internal_server("Path is not valid utf-8"))
}
