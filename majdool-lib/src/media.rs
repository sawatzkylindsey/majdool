use crate::database::MediaIndexDatabase;
use crate::util::compute_file_hash;
use std::path::PathBuf;

pub struct MediaSystem {
    index_db: MediaIndexDatabase,
    target_path: PathBuf,
}

impl MediaSystem {
    pub async fn flush_file(self, source: &PathBuf) {
        let hash = compute_file_hash(source).await.unwrap();
        match self.index_db.media_lookup(hash).await {
            Some(media) => {
                // Perform content wise comparison
            }
            None => {
                match self.index_db.media_insert().await {
                    Ok(id) => {
                        // woops
                    }
                    Err(e) => {
                        // Do something!
                    }
                }
            }
        }
    }
}
