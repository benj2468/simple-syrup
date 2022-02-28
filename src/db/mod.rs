use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::config::DBOptions;

pub async fn new_pool(db_options: &DBOptions) -> sqlx::Result<PgPool> {
    let DBOptions { uri } = db_options;
    PgPoolOptions::new().max_connections(5).connect(uri).await
}
