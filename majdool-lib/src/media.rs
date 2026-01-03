use crate::api::MediaId;
use crate::db::database::MediaIndexDatabase;
use crate::fs::filesystem::MediaFilesystem;
use crate::fs::fsutil::{FileHash, compute_file_hash, content_wise_equals};
use std::path::{Path, PathBuf};

pub struct MediaSystem {
    index_db: MediaIndexDatabase,
    filesystem: MediaFilesystem,
    target_path: PathBuf,
}

impl MediaSystem {
    pub async fn flush_file(&mut self, source: impl AsRef<Path>) -> Result<MediaId, ()> {
        // TODO: durability
        let hash = compute_file_hash(&source).await.map_err(|_| ())?;
        match self.index_db.media_lookup(hash).await {
            Some(media) => {
                // Perform content wise comparison
                if content_wise_equals(&source, media.path)
                    .await
                    .map_err(|_| ())?
                {
                    // We don't need to flush source - it's a duplicate.
                    Ok(media.id)
                } else {
                    // It's a hash collision - flush!
                    self.insert_flush_write(&hash, &source).await
                }
            }
            None => self.insert_flush_write(&hash, &source).await,
        }
    }

    async fn insert_flush_write(
        &mut self,
        hash: &FileHash,
        source: impl AsRef<Path>,
    ) -> Result<MediaId, ()> {
        // TODO: durability
        match self.index_db.media_insert(hash).await {
            Ok(id) => match self.filesystem.flush_write(&source, id).await {
                Ok(_) => self.index_db.media_sync(id, source).await.map(|_| id),
                Err(_) => Err(()),
            },
            Err(_) => {
                // Do something!
                Err(())
            }
        }
    }
}
