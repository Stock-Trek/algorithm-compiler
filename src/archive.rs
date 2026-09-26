use crate::error::{ACError, ACResult};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use std::{fs, path::Path};

pub struct Archive;

impl Archive {
    pub fn create(source_dir: &Path, archive: &Path) -> ACResult<()> {
        if let Some(parent) = archive.parent() {
            fs::create_dir_all(parent).map_err(ACError::FileSystem)?;
        }
        let file = fs::File::create(archive).map_err(ACError::FileSystem)?;
        let encoder = GzEncoder::new(file, Compression::default());
        let mut builder = tar::Builder::new(encoder);
        builder.follow_symlinks(false);
        builder
            .append_dir_all(".", source_dir)
            .map_err(ACError::FileSystem)?;
        let encoder = builder.into_inner().map_err(ACError::FileSystem)?;
        encoder.finish().map_err(ACError::FileSystem)?;
        Ok(())
    }

    pub fn extract(archive: &Path, sink_dir: &Path) -> ACResult<()> {
        let file = fs::File::open(archive).map_err(ACError::FileSystem)?;
        let decoder = GzDecoder::new(file);
        let mut builder = tar::Archive::new(decoder);
        builder.unpack(sink_dir).map_err(ACError::FileSystem)?;
        Ok(())
    }
}
