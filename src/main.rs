use actix_cors::Cors;
use actix_web::{middleware, App, HttpServer};
use config::Config;
use env_logger::Env;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

use crate::config::{SSLData, Server, ServerPublicData};
mod api;
mod config;
mod db;

macro_rules! start_server {
    ($app:ident, $mod:ident, $pool:ident) => {
        $app.app_data(api::$mod::server_builder($pool.clone()))
            .service(api::index)
            .service(api::$mod::server_ty)
            .service(api::$mod::register)
            .service(api::$mod::register_check)
            .service(api::$mod::auth)
            .service(api::$mod::auth_check)
            .service(api::$mod::status_check)
    };
}

async fn start_server(server: Server) -> std::io::Result<()> {
    let Server {
        host,
        database,
        port,
        server_ty,
        ssl_data,
    } = server;

    let SSLData {
        ssl_cert_file,
        ssl_key_file,
    } = ssl_data;

    sqlx::migrate!()
        .run(&database)
        .await
        .expect("There was an error running the migration");

    println!("[{:?}]: {}:{}", server_ty, host, port);

    let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls_server())?;
    builder
        .set_private_key_file(&ssl_key_file, SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file(&ssl_cert_file).unwrap();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_header()
            .allow_any_origin()
            .allow_any_method();

        let app = App::new().wrap(cors).wrap(middleware::Logger::default());
        match server_ty {
            config::ServerType::Email => start_server!(app, email, database),
        }
    })
    .bind_openssl(format!("{}:{}", host, port), builder)?
    .run()
    .await
}

async fn root_server(root: &Config) -> std::io::Result<()> {
    let Config {
        host,
        port,
        ssl_data,
        servers,
    } = root;

    let SSLData {
        ssl_cert_file,
        ssl_key_file,
    } = ssl_data;

    println!("[root]: {}:{}", host, port);

    let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls_server())?;
    builder
        .set_private_key_file(&ssl_key_file, SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file(&ssl_cert_file).unwrap();

    let servers: Vec<ServerPublicData> = servers.iter().map(|x| x.into()).collect();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_header()
            .allow_any_origin()
            .allow_any_method();

        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .app_data(servers.clone())
            .service(config::root)
    })
    .bind_openssl(format!("{}:{}", host, port), builder)?
    .run()
    .await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    dotenv::dotenv().expect("Cannot initiate server without env variables.");

    let config = config::Config::new().await;

    let servers = config.servers.clone();

    let root = root_server(&config);

    let servers = servers.into_iter().map(start_server);

    let (_, _) = futures::future::join(futures::future::join_all(servers), root).await;

    Ok(())
}
