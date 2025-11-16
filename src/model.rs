use sea_query::{ColumnDef, ColumnType, Iden, Order, PostgresQueryBuilder, Query, Table};

#[derive(Iden)]
pub enum Media {
    Table,
    Id,
    Path,
    Synced,
    Lost,
}

#[derive(sqlx::FromRow, Debug)]
pub struct MediaView {
    id: i32,
    path: String,
}
