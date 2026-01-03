use crate::fs::fsutil::FileHash;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug)]
pub struct MediaId {
    pub value: i64,
}

impl MediaId {
    pub fn new(value: i64) -> Self {
        Self { value }
    }
}

impl MediaId {
    pub fn file_base(self) -> String {
        hex::encode(self.value.to_be_bytes())
    }
}

#[derive(Debug)]
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
