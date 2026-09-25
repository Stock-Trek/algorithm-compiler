use crate::{
    error::{ACError, ACResult},
    files::Files,
    program::Program,
};
use std::{fs, path::PathBuf};

pub struct Archive {}

impl Archive {
    pub fn create(source_dir: &PathBuf, archive: &PathBuf) -> ACResult<String> {
        if let Some(parent) = archive.parent() {
            fs::create_dir_all(parent).map_err(ACError::FileSystem)?;
        }
        Program::run(
            "tar",
            &[
                "czf",
                Files::path_str(archive)?,
                "-C",
                Files::path_str(source_dir)?,
                ".",
            ],
            source_dir,
        )
    }

    pub fn extract(archive: &PathBuf, sink_dir: &PathBuf) -> ACResult<String> {
        Program::run(
            "tar",
            &[
                "xzf",
                Files::path_str(archive)?,
                "-C",
                Files::path_str(sink_dir)?,
            ],
            sink_dir,
        )
    }
}
