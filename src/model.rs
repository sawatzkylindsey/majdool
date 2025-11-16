use sea_query::{Iden, PostgresQueryBuilder, Query};
use sea_query_sqlx::SqlxBinder;
use sqlx::PgPool;

#[derive(Iden)]
#[allow(dead_code)]
pub enum Media {
    Table,
    Id,
    Path,
    Synced,
    Lost,
}

#[derive(sqlx::FromRow, Debug)]
#[allow(dead_code)]
pub struct MediaView {
    id: i32,
    path: String,
}


pub async fn poc_psql() {
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

    println!("verified db connectivity..")
}

