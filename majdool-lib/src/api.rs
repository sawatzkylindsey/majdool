use crate::fs::fsutil::FileHash;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub struct MediaId {
    value: i64,
}

impl MediaId {
    pub fn file_base(self) -> String {
        hex::encode(self.value.to_be_bytes())
    }
}

pub struct Media {
    pub id: MediaId,
    pub path: PathBuf,
    pub hash: FileHash,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_id() {
        let id = MediaId { value: 1 };
        assert_eq!(id.file_base(), "0000000000000001");

        let id = MediaId { value: i64::MAX };
        assert_eq!(id.file_base(), "7fffffffffffffff");
    }
}
