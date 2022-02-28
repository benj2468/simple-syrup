use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() {
    let uri: String = std::env::var("DATABASE_URL").expect("Must supply DATABASE_URL");

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&uri)
        .await
        .expect("Could not connect to db");

    sqlx::migrate!()
        .run(&db)
        .await
        .expect("There was an error running the migration");
}
