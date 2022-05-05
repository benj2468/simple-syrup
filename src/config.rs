use crate::db;
use actix_web::{HttpRequest, HttpResponseBuilder, Responder};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Clone, Copy, Deserialize, Serialize, Debug)]
pub(crate) enum ServerType {
    Email,
    QA,
    Password,
    Biometric,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct DBOptions {
    pub(crate) uri: String,
}

#[derive(Clone)]
pub(crate) struct Server {
    pub(crate) _dev_port: u32,
    pub(crate) database: PgPool,
    pub(crate) server_ty: ServerType,
}

#[derive(Clone)]
pub struct Config {
    pub(crate) server: Server,
    pub(crate) host: String,
    pub(crate) port: u32,
    pub(crate) active_servers: Vec<ServerPublicData>,
}

impl Config {
    #[cfg(test)]
    pub(crate) async fn test(server_ty: ServerType) -> Self {
        let active_server = ServerPublicData {
            server_ty,
            url: "https://server.test:8080".into(),
        };

        let db = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let database = db::new_pool(&DBOptions { uri: db })
            .await
            .expect("Could not connect to test pool");

        sqlx::migrate!()
            .run(&database)
            .await
            .expect("Could not perform db migrations");

        sqlx::query!("DELETE FROM authenticated")
            .execute(&database)
            .await
            .expect("Error clearing database");

        let server = Server {
            _dev_port: 0000,
            database,
            server_ty,
        };

        Self {
            server,
            host: "".into(),
            port: 0,
            active_servers: vec![active_server],
        }
    }

    pub async fn new() -> Self {
        let port: u32 = std::env::var("PORT")
            .expect("Must supply PORT")
            .parse()
            .expect("Expected a positive integer for the Root Port");

        let host: String = std::env::var("HOST").expect("Must supply HOST");
        let db_uri: String = std::env::var("DATABASE_URL").expect("Must supply HOST");
        let server_ty: ServerType =
            serde_json::from_str(&std::env::var("SERVER_TY").expect("Must supply SERVER_TY"))
                .expect("SERVER_TY not correctly formatted");

        let active_servers: Vec<ServerPublicData> = serde_json::from_str(
            &std::env::var("ACTIVE_SERVERS").expect("Must supply ACTIVE_SERVERS"),
        )
        .expect("Active servers not correctly formatted");

        let database = db::new_pool(&DBOptions { uri: db_uri })
            .await
            .expect("could not connect to db");

        let server = Server {
            database,
            server_ty,
            _dev_port: port,
        };

        Self {
            host,
            server,
            port,
            active_servers,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerPublicData {
    url: String,
    server_ty: ServerType,
}

#[actix_web::get("/")]
pub async fn root(req: HttpRequest) -> impl Responder {
    let servers = req.app_data::<Vec<ServerPublicData>>().unwrap();

    HttpResponseBuilder::new(StatusCode::OK).json(servers)
}
