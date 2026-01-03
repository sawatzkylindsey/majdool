use crate::api::{Media, MediaId};
use sea_query::Iden;

#[derive(Iden)]
#[allow(dead_code)]
pub enum MediaIndex {
    Table,
    Id,
    Path,
    Hash,
    Synced,
    Lost,
}

#[derive(sqlx::FromRow, Debug)]
#[allow(dead_code)]
pub struct MediaIndexView {
    // BIGSERIAL is represented as an i64 (it truncates out the negative half of the id space).
    id: i64,
    path: Option<String>,
    hash: [u8; 32],
}

impl From<MediaIndexView> for Media {
    fn from(value: MediaIndexView) -> Self {
        Self {
            id: MediaId { value: value.id },
            path: value.path.unwrap().into(),
            hash: value.hash,
        }
    }
}
