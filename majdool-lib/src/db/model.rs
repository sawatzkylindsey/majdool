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
