use std::path::PathBuf;

struct MediaFilesystem {
    root: PathBuf,
}

impl MediaFilesystem {
    async fn write(&self, source: &PathBuf) {}
}
