use sea_query::{Iden, PostgresQueryBuilder, Query};
use sea_query_sqlx::SqlxBinder;
use sqlx::pool::PoolConnection;
use sqlx::{PgPool, Postgres};

#[derive(Iden)]
#[allow(dead_code)]
pub enum MediaIndex {
    Table,
    Id,
    Path,
    Synced,
    Lost,
}

#[derive(sqlx::FromRow, Debug)]
#[allow(dead_code)]
pub struct MediaIndexView {
    // BIGSERIAL is represented as an i64 (it truncates out the negative half of the id space).
    id: i64,
    path: String,
}

#[derive(Clone, Copy)]
pub struct MediaId {
    value: i64,
}

impl MediaId {
    pub fn file_base(self) -> String {
        hex::encode(self.value.to_be_bytes())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_id() {
        let id = MediaId {
            value: 1,
        };
        assert_eq!(id.file_base(), "0000000000000001");

        let id = MediaId {
            value: i64::MAX,
        };
        assert_eq!(id.file_base(), "7fffffffffffffff");
    }
}
