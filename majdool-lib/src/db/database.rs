use crate::api::{Media, MediaId};
use crate::db::model::{MediaIndex, MediaIndexView};
use crate::fs::fsutil::FileHash;
use sea_query::{Expr, ExprTrait, PostgresQueryBuilder, Query};
use sea_query_sqlx::SqlxBinder;
use sqlx::pool::PoolConnection;
use sqlx::{PgPool, Postgres};
use std::path::Path;

pub struct MediaIndexDatabase {
    pool: PoolConnection<Postgres>,
}

impl MediaIndexDatabase {
    pub async fn media_lookup(&mut self, hash: FileHash) -> Option<Media> {
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
            .fetch_one(&mut *self.pool)
            .await;

        match row {
            Ok(miv) => Some(Media::from(miv)),
            Err(_) => None,
        }
    }

    pub async fn media_insert(&mut self, hash: &FileHash) -> Result<MediaId, ()> {
        let (sql, values) = Query::insert()
            .into_table(MediaIndex::Table)
            .columns([MediaIndex::Hash, MediaIndex::Synced, MediaIndex::Lost])
            .values_panic([hash.as_ref().into(), false.into(), false.into()])
            .returning_col(MediaIndex::Id)
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_as_with::<_, (i64,), _>(&sql, values)
            .fetch_one(&mut *self.pool)
            .await
            .map(|i| MediaId::new(i.0))
            .map_err(|_| ())
    }

    pub async fn media_sync(&mut self, id: MediaId, path: impl AsRef<Path>) -> Result<(), ()> {
        let (sql, values) = Query::update()
            .table(MediaIndex::Table)
            .values([
                (MediaIndex::Path, path.as_ref().to_str().into()),
                (MediaIndex::Synced, true.into()),
            ])
            .and_where(Expr::col(MediaIndex::Id).eq(id.value))
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&mut *self.pool)
            .await
            .map(|_| ())
            .map_err(|_| ())
    }
}

pub async fn tmp_initialize() -> MediaIndexDatabase {
    let connection = PgPool::connect("postgres://lindsey@127.0.0.1/majdool")
        .await
        .unwrap();
    let pool = connection.try_acquire().unwrap();
    MediaIndexDatabase { pool }
}

// WIP
#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;
    use testcontainers_modules::{postgres, testcontainers::runners::AsyncRunner};

    #[tokio::test]
    async fn test_with_postgres() {
        let container = postgres::Postgres::default().start().await.unwrap();
        let host_ip = container.get_host().await.unwrap();
        let host_port = container.get_host_port_ipv4(5432).await.unwrap();

        // Build connection string
        let connection_string = format!(
            "postgres://postgres:postgres@{}:{}/postgres",
            host_ip, host_port
        );

        // Create connection pool
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .unwrap();

        // Run migrations (if you have them)
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // Now you can use the pool for your tests
        let hash = [0u8; 32];
    }
    //
    // use testcontainers::clients;
    // use sqlx::postgres::PgPoolOptions;
    // use testcontainers_modules::postgres::Postgres;
    //
    // #[tokio::test]
    // async fn test_insert_media_index() {
    //     let docker = clients::Cli::default();
    //     let postgres = docker.run(Postgres::default());
    //
    //     let connection_string = format!(
    //         "postgres://postgres:postgres@127.0.0.1:{}/postgres",
    //         postgres.get_host_port_ipv4(5432)
    //     );
    //
    //     let pool = PgPoolOptions::new()
    //         .max_connections(5)
    //         .connect(&connection_string)
    //         .await
    //         .unwrap();
    //
    //     // Run migrations
    //     sqlx::migrate!("./migrations")
    //         .run(&pool)
    //         .await
    //         .unwrap();
    //
    //     // Your test code here
    //     let hash = [0u8; 32];
    //     // ... rest of test
    // }
}
