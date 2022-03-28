#![feature(bool_to_option)]
use actix_cors::Cors;
use actix_web::{middleware, App, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use config::{Config, Server};
use env_logger::Env;

mod api;
mod auth;
mod config;
mod db;

macro_rules! build_app {
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

async fn root_server(root: Config) -> std::io::Result<()> {
    let Config {
        host,
        port,
        server,
        active_servers,
    } = root;

    println!("[root]: {}:{}", host, port);

    sqlx::migrate!()
        .run(&server.database)
        .await
        .expect("Could not perform db migrations");

    let _host = host.clone();

    let Server {
        database,
        server_ty,
        ..
    } = server;

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_header()
            .allow_any_origin()
            .allow_any_method();

        let _auth_middleware = HttpAuthentication::bearer(auth::validator);

        let app = App::new()
            // Reset this when we are ready to implement JWT requirements
            // .wrap(auth_middleware)
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .app_data(active_servers.clone())
            .service(config::root);

        match server_ty {
            #[cfg(feature = "email")]
            config::ServerType::Email => build_app!(app, email, database),
            #[cfg(feature = "qa")]
            config::ServerType::QA => build_app!(app, qa, database),
            #[allow(unreachable_patterns)]
            _ => app,
        }
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
