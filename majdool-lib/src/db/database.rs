use crate::api::{Media, MediaId};
use crate::db::model::{MediaIndex, MediaIndexView};
use crate::fs::fsutil::FileHash;
use sea_query::{Expr, ExprTrait, PostgresQueryBuilder, Query};
use sea_query_sqlx::SqlxBinder;
use sqlx::{PgPool, Postgres};
use std::path::Path;

pub struct MediaIndexDatabase {
    pool: sqlx::Pool<Postgres>,
}

impl MediaIndexDatabase {
    pub async fn media_lookup(&self, hash: FileHash) -> Option<Media> {
        let (sql, values) = Query::select()
            .from(MediaIndex::Table)
            .column(MediaIndex::Id)
            .column(MediaIndex::Path)
            .column(MediaIndex::Hash)
            .and_where(Expr::col(MediaIndex::Hash).eq(hash.as_slice()))
            .and_where(Expr::col(MediaIndex::Synced).eq(true))
            .and_where(Expr::col(MediaIndex::Lost).eq(false))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_as_with::<_, MediaIndexView, _>(&sql, values.clone())
            .fetch_one(&self.pool)
            .await;

        match row {
            Ok(miv) => Some(Media::from(miv)),
            Err(_) => None,
        }
    }

    pub async fn media_insert(&self, hash: &FileHash) -> Result<MediaId, ()> {
        let (sql, values) = Query::insert()
            .into_table(MediaIndex::Table)
            .columns([MediaIndex::Hash, MediaIndex::Synced, MediaIndex::Lost])
            .values_panic([hash.as_ref().into(), false.into(), false.into()])
            .returning_col(MediaIndex::Id)
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_as_with::<_, (i64,), _>(&sql, values)
            .fetch_one(&self.pool)
            .await
            .map(|i| MediaId::new(i.0))
            .map_err(|_| ())
    }

    pub async fn media_sync(&self, id: MediaId, path: impl AsRef<Path>) -> Result<(), ()> {
        let (sql, values) = Query::update()
            .table(MediaIndex::Table)
            .values([
                (MediaIndex::Path, path.as_ref().to_str().into()),
                (MediaIndex::Synced, true.into()),
            ])
            .and_where(Expr::col(MediaIndex::Id).eq(id.value))
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|_| ())
    }
}

pub async fn tmp_initialize() -> MediaIndexDatabase {
    let pool = PgPool::connect("postgres://lindsey@127.0.0.1/majdool")
        .await
        .unwrap();
    MediaIndexDatabase { pool }
}

#[cfg(test)]
mod tests {
    use crate::api::{Media, MediaId};
    use crate::db::database::MediaIndexDatabase;
    use crate::fs::fsutil::FileHash;
    use futures::sink::drain;
    use rand::RngCore;
    use sqlx::Postgres;
    use sqlx::pool::PoolConnection;
    use sqlx::postgres::PgPoolOptions;
    use testcontainers::ContainerAsync;
    use testcontainers_modules::{postgres, testcontainers::runners::AsyncRunner};
    use tokio::sync::OnceCell;
    use tokio_test::assert_err;

    struct TestDb {
        mid: MediaIndexDatabase,
        _container: ContainerAsync<postgres::Postgres>,
    }

    async fn test_db() -> TestDb {
        let container = postgres::Postgres::default().start().await.unwrap();
        let host_ip = container.get_host().await.unwrap();
        let host_port = container.get_host_port_ipv4(5432).await.unwrap();
        let connection_string = format!(
            "postgres://postgres:postgres@{}:{}/postgres",
            host_ip, host_port
        );
        let pool: sqlx::Pool<Postgres> = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        TestDb {
            mid: MediaIndexDatabase { pool },
            _container: container,
        }
    }

    #[tokio::test]
    async fn media_lookup() {
        // Setup
        let test_db = test_db().await;
        let mid = test_db.mid;
        let hash = random_hash();
        let path = "/some/path";

        // Execute & verify
        assert!(mid.media_lookup(hash).await.is_none());
        let result = mid.media_insert(&hash).await.unwrap();

        assert!(mid.media_lookup(hash).await.is_none());
        let media_id = MediaId {
            value: result.value,
        };

        // Only after syncing does it show up.
        mid.media_sync(media_id, path).await.unwrap();
        let result = mid.media_lookup(hash).await.unwrap();
        assert_eq!(result.id, media_id);
        assert_eq!(result.hash, hash);
        assert_eq!(result.path.to_str().unwrap(), path);
    }

    #[tokio::test]
    async fn duplicate_media_insert() {
        // Setup
        let test_db = test_db().await;
        let mid = test_db.mid;
        let hash = random_hash();
        let path = "/some/path";
        let result1 = mid.media_insert(&hash).await.unwrap();

        // Execute
        let result2 = mid.media_insert(&hash).await.unwrap();

        // Verify
        assert_ne!(result2, result1);
    }

    #[tokio::test]
    async fn duplicate_media_sync() {
        // Setup
        let test_db = test_db().await;
        let mid = test_db.mid;
        let hash = random_hash();
        let path = "/some/path";
        let id_1 = mid.media_insert(&hash).await.unwrap();
        let id_2 = mid.media_insert(&hash).await.unwrap();
        mid.media_sync(id_1, path).await.unwrap();

        // Execute
        let result = mid.media_sync(id_2, path).await;

        // Verify
        assert!(result.is_err());
    }

    fn random_hash() -> FileHash {
        let mut rng = rand::thread_rng();
        let mut hash = [0u8; 32];
        rng.fill_bytes(&mut hash);
        hash
    }
}
