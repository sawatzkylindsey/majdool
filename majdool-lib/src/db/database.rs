use crate::api::{Media, MediaId};
use crate::db::model::{MediaIndex, MediaIndexView};
use crate::fs::fsutil::FileHash;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_sqlx::SqlxBinder;
use sqlx::pool::PoolConnection;
use sqlx::{PgPool, Postgres};

pub struct MediaIndexDatabase {
    pool: PoolConnection<Postgres>,
}

impl MediaIndexDatabase {
    pub async fn media_lookup(&self, hash: FileHash) -> Option<Media> {
        todo!()
    }
}

impl MediaIndexDatabase {
    pub async fn media_insert(&self) -> Result<MediaId, ()> {
        todo!()
    }
}

pub async fn tmp_initialize() -> MediaIndexDatabase {
    let connection = PgPool::connect("postgres://lindsey@127.0.0.1/majdool")
        .await
        .unwrap();
    let mut pool = connection.try_acquire().unwrap();
    MediaIndexDatabase { pool }
}

impl MediaIndexDatabase {
    async fn abc(&mut self) {
        let (sql, values) = Query::select()
            .column(MediaIndex::Id)
            .column(MediaIndex::Path)
            .from(MediaIndex::Table)
            .build_sqlx(PostgresQueryBuilder);

        let rows = sqlx::query_as_with::<_, MediaIndexView, _>(&sql, values.clone())
            .fetch_all(&mut *self.pool)
            .await
            .unwrap();

        for row in rows {
            println!("{row:?}");
        }
    }
}
