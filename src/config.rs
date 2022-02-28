use crate::db;
use actix_web::{HttpRequest, HttpResponseBuilder, Responder};
use hyper::StatusCode;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Clone, Copy, Deserialize, Serialize, Debug)]
pub(crate) enum ServerType {
    Email,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct DBOptions {
    pub(crate) uri: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ConfigServer {
    pub(crate) port: u32,
    pub(crate) server_ty: ServerType,
    pub(crate) db_options: DBOptions,
}

#[derive(Clone)]
pub struct SSLData {
    pub(crate) ssl_cert_file: String,
    pub(crate) ssl_key_file: String,
}

#[derive(Clone)]
pub(crate) struct Server {
    pub(crate) ssl_data: SSLData,
    pub(crate) host: String,
    pub(crate) port: u32,
    pub(crate) database: PgPool,
    pub(crate) server_ty: ServerType,
}

#[derive(Clone)]
pub struct Config {
    pub(crate) servers: Vec<Server>,
    pub(crate) ssl_data: SSLData,
    pub(crate) host: String,
    pub(crate) port: u32,
}

impl Config {
    pub async fn new() -> Config {
        let config_servers = std::env::var("SERVERS_CONFIG").expect("Must supply SERVERS_CONFIG");
        let config_servers: Vec<ConfigServer> =
            serde_json::from_str(&config_servers).expect("Invalid Servers Config");

        let root_port: u32 = std::env::var("ROOT_PORT")
            .expect("Must supply ROOT_PORT")
            .parse()
            .expect("Expected a positive integer for the Root Port");

        let host: String = std::env::var("HOST").expect("Must supply HOST");

        assert!(
            config_servers
                .iter()
                .map(|s| s.port)
                .chain(vec![root_port].into_iter())
                .all_unique(),
            "MUST PROVIDE ALL UNIQUE PORTS"
        );

        let dbs = futures::future::join_all(
            config_servers
                .iter()
                .map(|s| &s.db_options)
                .map(db::new_pool),
        )
        .await
        .into_iter()
        .collect::<sqlx::Result<Vec<PgPool>>>()
        .expect("Unable to connect to some Databases on load");

        let ssl_cert_file = std::env::var("SSL_CERT_FILE").expect("Must supply SSL_CERT_FILE");
        let ssl_key_file = std::env::var("SSL_KEY_FILE").expect("Must supply SSL_KEY_FILE");

        let ssl_data = SSLData {
            ssl_cert_file,
            ssl_key_file,
        };

        let servers: Vec<Server> = config_servers
            .iter()
            .zip(dbs)
            .map(|(config, database)| Server {
                database,
                host: host.clone(),
                server_ty: config.server_ty,
                port: config.port,
                ssl_data: ssl_data.clone(),
            })
            .collect();

        Config {
            host,
            servers,
            ssl_data,
            port: root_port,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ServerPublicData {
    url: String,
    server_ty: ServerType,
}

impl From<&Server> for ServerPublicData {
    fn from(server: &Server) -> Self {
        ServerPublicData {
            url: format!("https://{}:{}", server.host, server.port),
            server_ty: server.server_ty,
        }
    }
}

#[actix_web::get("/")]
pub async fn root(req: HttpRequest) -> impl Responder {
    let servers = req.app_data::<Vec<ServerPublicData>>().unwrap();

    HttpResponseBuilder::new(StatusCode::OK).json(servers)
}
