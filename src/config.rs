use crate::db;
use actix_web::{HttpRequest, HttpResponseBuilder, Responder};
use hyper::StatusCode;
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
    pub(crate) server_ty: ServerType,
    pub(crate) db_options: DBOptions,
}

#[derive(Clone)]
pub(crate) struct Server {
    pub(crate) host: String,
    pub(crate) _dev_port: u32,
    pub(crate) database: PgPool,
    pub(crate) server_ty: ServerType,
}

#[derive(Clone)]
pub struct Config {
    pub(crate) servers: Vec<Server>,
    pub(crate) host: String,
    pub(crate) port: u32,
}

impl Config {
    pub async fn new() -> Config {
        let config_servers = std::env::var("SERVERS_CONFIG").expect("Must supply SERVERS_CONFIG");
        let config_servers: Vec<ConfigServer> =
            serde_json::from_str(&config_servers).expect("Invalid Servers Config");

        let server_id: usize = std::env::var("SERVER_ID")
            .expect("Must supply a SERVER_ID")
            .parse()
            .expect("SERVER_ID must be an integer");

        let server: ConfigServer = config_servers
            .get(server_id)
            .expect("Invalid SERVER_ID for server data")
            .clone();

        let servers = vec![server];

        let port: u32 = std::env::var("PORT")
            .expect("Must supply PORT")
            .parse()
            .expect("Expected a positive integer for the Root Port");

        let host: String = std::env::var("HOST").expect("Must supply HOST");

        let dbs =
            futures::future::join_all(servers.iter().map(|s| &s.db_options).map(db::new_pool))
                .await
                .into_iter()
                .collect::<sqlx::Result<Vec<PgPool>>>()
                .expect("Unable to connect to some Databases on load");

        let servers: Vec<Server> = servers
            .iter()
            .zip(dbs)
            .map(|(config, database)| Server {
                database,
                host: host.clone(),
                server_ty: config.server_ty,
                _dev_port: port,
            })
            .collect();

        Config {
            host,
            servers,
            port,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ServerPublicData {
    url: String,
    server_ty: ServerType,
}

impl ServerPublicData {
    pub(crate) fn new(i: usize, server: &Server) -> Self {
        let Server {
            server_ty, host, ..
        } = server;
        let name = format!("{:?}{}", server_ty, i).to_lowercase();
        let url = format!("https://{}/{}/", host, name);
        println!("[{:?}]: {}", server_ty, url);
        ServerPublicData {
            url,
            server_ty: *server_ty,
        }
    }
}

#[actix_web::get("/")]
pub async fn root(req: HttpRequest) -> impl Responder {
    let servers = req.app_data::<Vec<ServerPublicData>>().unwrap();

    HttpResponseBuilder::new(StatusCode::OK).json(servers)
}
