use actix_cors::Cors;
use actix_web::{middleware, App, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use config::{Config, Server, ServerPublicData};
use env_logger::Env;
use futures::future::join_all;
use sqlx::migrate::MigrateError;

mod api;
mod auth;
mod config;
mod db;

macro_rules! build_scope {
    ($name:ident, $mod:ident, $pool:ident) => {
        actix_web::Scope::new(&$name)
            .app_data(api::$mod::server_builder($pool.clone()))
            .service(api::index)
            .service(api::$mod::server_ty)
            .service(api::$mod::register)
            .service(api::$mod::register_check)
            .service(api::$mod::auth)
            .service(api::$mod::auth_check)
            .service(api::$mod::status_check)
    };
}

async fn root_server(root: Config) -> std::io::Result<()> {
    let Config {
        host,
        port,
        servers,
    } = root;

    println!("[root]: {}:{}", host, port);

    let servers_pub: Vec<ServerPublicData> = servers
        .clone()
        .iter()
        .enumerate()
        .map(|(i, x)| ServerPublicData::new(i, x))
        .collect();

    join_all(
        servers
            .iter()
            .map(|server| sqlx::migrate!().run(&server.database)),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<()>, MigrateError>>()
    .expect("Could not perform db migrations");

    let _host = host.clone();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_header()
            .allow_any_origin()
            .allow_any_method();

        let auth_middleware = HttpAuthentication::bearer(auth::validator);

        let mut app = App::new()
            // Reset this when we are ready to implement JWT requirements
            // .wrap(auth_middleware)
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .app_data(servers_pub.clone())
            .service(config::root);

        for (i, server) in servers.iter().enumerate() {
            let Server {
                database,
                server_ty,
                ..
            } = server;

            let name = format!("{:?}{}", server_ty, i).to_lowercase();

            let scope = match server_ty {
                config::ServerType::Email => build_scope!(name, email, database),
            };

            app = app.service(scope);
        }

        app
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    if cfg!(debug_assertions) {
        dotenv::dotenv().expect("Cannot initiate server without env variables.");
    }

    let config = config::Config::new().await;

    root_server(config).await
}
