use crate::api::MediaId;
use crate::fs::fsutil::copy_file;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

const FLUSH: &'static str = "flush/";
const DEFAULT_EXTENSION: &'static str = "unk";

pub struct MediaFilesystem {
    root: PathBuf,
}

impl MediaFilesystem {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        if !root.is_absolute() {
            return Err(format!("root must be absolute: {}", root.display()));
        }

        let flush_dir = root.join(FLUSH);

        if !flush_dir.exists() {
            std::fs::create_dir(flush_dir).map_err(|e| e.to_string())?;
        }

        Ok(Self { root })
    }

    pub async fn flush_write(
        &self,
        source: impl AsRef<Path>,
        id: MediaId,
    ) -> Result<PathBuf, String> {
        // We need to manually retain the extension for the file, because we're writing it to a path based off its Id (not its source name).
        let extension = source
            .as_ref()
            .extension()
            .unwrap_or(&OsStr::new(DEFAULT_EXTENSION));
        let mut destination = self.root.join(FLUSH).join(id.file_base());
        destination.set_extension(&extension);
        copy_file(source, &destination)
            .await
            .map_err(|e| e.to_string())?;
        Ok(destination)
    }
}
