use crate::api::MediaId;
use crate::db::database::MediaIndexDatabase;
use crate::fs::filesystem::MediaFilesystem;
use crate::fs::fsutil::{FileHash, compute_file_hash, content_wise_equals};
use std::path::Path;

pub struct MediaSystem {
    index_db: MediaIndexDatabase,
    filesystem: MediaFilesystem,
}

impl MediaSystem {
    pub fn new(index_db: MediaIndexDatabase, filesystem: MediaFilesystem) -> Self {
        Self {
            index_db,
            filesystem,
        }
    }

    pub async fn flush_file(&self, source: impl AsRef<Path>) -> Result<MediaId, String> {
        // TODO: durability
        let hash = compute_file_hash(&source)
            .await
            .map_err(|e| e.to_string())?;
        match self.index_db.media_lookup(hash).await {
            Ok(Some(media)) => {
                // Perform content wise comparison
                if content_wise_equals(&source, media.path)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    // We don't need to flush source - it's a duplicate.
                    Ok(media.id)
                } else {
                    // It's a hash collision - flush!
                    self.insert_flush_write(&hash, &source).await
                }
            }
            Ok(None) => self.insert_flush_write(&hash, &source).await,
            Err(e) => Err(e),
        }
    }

    async fn insert_flush_write(
        &self,
        hash: &FileHash,
        source: impl AsRef<Path>,
    ) -> Result<MediaId, String> {
        // TODO: durability
        match self.index_db.media_insert(hash).await {
            Ok(id) => match self.filesystem.flush_write(&source, id).await {
                Ok(destination) => self.index_db.media_sync(id, destination).await.map(|_| id),
                Err(e) => Err(e),
            },
            Err(e) => Err(e.to_string()),
        }
    }
}
