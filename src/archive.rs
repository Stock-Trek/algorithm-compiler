use crate::{
    error::{ACError, ACResult},
    files::Files,
    program::Program,
};
use std::{fs, path::Path};

pub struct Archive {}

impl Archive {
    pub fn create(source_dir: &Path, archive: &Path) -> ACResult<String> {
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

    pub fn extract(archive: &Path, sink_dir: &Path) -> ACResult<String> {
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
