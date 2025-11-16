mod model;
use model::Media;

// use tokio_postgres::{NoTls, Error};
use sea_query::{ColumnDef, ColumnType, Iden, Order, PostgresQueryBuilder, Query, Table};
use sea_query_sqlx::SqlxBinder;
use sqlx::PgPool;
use crate::model::MediaView;

#[tokio::main]
async fn main() {
    print!("Hello world!");

    let connection = PgPool::connect("postgres://lindsey@127.0.0.1/majdool")
        .await
        .unwrap();
    let mut pool = connection.try_acquire().unwrap();

    let (sql, values) = Query::select()
        .column(Media::Id)
        .column(Media::Path)
        .from(Media::Table)
        .build_sqlx(PostgresQueryBuilder);

    let rows = sqlx::query_as_with::<_, MediaView, _>(&sql, values.clone())
        .fetch_all(&mut *pool)
        .await
        .unwrap();

    for row in rows {
        println!("{row:?}");
    }

    print!("Doners!");
}
