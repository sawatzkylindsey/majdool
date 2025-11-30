use crate::database::MediaIndexDatabase;
use crate::filesystem::MediaFilesystem;
use crate::fsutil::compute_file_hash;
use std::path::{Path, PathBuf};
use crate::model::MediaId;

pub struct MediaSystem {
    index_db: MediaIndexDatabase,
    filesystem: MediaFilesystem,
    target_path: PathBuf,
}

impl MediaSystem {
    pub async fn flush_file(self, source: impl AsRef<Path>) -> Result<MediaId, ()> {
        let hash = compute_file_hash(&source).await.unwrap();
        match self.index_db.media_lookup(hash).await {
            Some(media) => {
                // Perform content wise comparison
                if content_wise_equals(&source, media.path) {
                    // We don't need to flush source - it's a duplicate.
                    Ok(media.id)
                } else {
                    // It's a hash collision - flush!
                    self.insert_flush_write(&source)
                }
            }
            None => {
                self.insert_flush_write(&source)
            }
        }
    }

    async fn insert_flush_write(self, source: impl AsRef<Path>) -> Result<MediaId, ()> {
        match self.index_db.media_insert().await {
            Ok(id) => {
                self.filesystem.flush_write(source, id).await.map(|_| id)
            }
            Err(_) => {
                // Do something!
                Err(())
            }
        }
    }
}
