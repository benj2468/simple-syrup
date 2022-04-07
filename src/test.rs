use super::*;

use actix_web::test;

macro_rules! build_test_app {
    ($config:ident) => {
        actix_web::test::init_service({
            let crate::config::Config {
                host,
                server,
                active_servers,
                ..
            } = $config;

            let _host = host.clone();

            let crate::config::Server {
                database,
                server_ty: other_server_ty,
                ..
            } = server;

            let app = actix_web::App::new()
                .wrap(actix_web::middleware::Logger::default())
                .app_data(active_servers.clone())
                .service(crate::config::root);

            match other_server_ty {
                crate::config::ServerType::Email => crate::build_app_ty!(app, email, database),
                crate::config::ServerType::QA => crate::build_app_ty!(app, qa, database),
                crate::config::ServerType::Password => {
                    crate::build_app_ty!(app, password, database)
                }
                #[allow(unreachable_patterns)]
                _ => app,
            }
        })
    };
}

pub type TestApp = Config;

impl TestApp {
    pub(crate) async fn _get_base(&self) -> Vec<config::ServerPublicData> {
        let app = build_test_app!(self).await;

        let req = test::TestRequest::get().uri("/").to_request();

        test::call_and_read_body_json(&app, req).await
    }

    pub(crate) async fn _get_server_ty(&self) -> config::ServerType {
        let app = build_test_app!(self).await;
        let req = test::TestRequest::get().uri("/ty").to_request();

        test::call_and_read_body_json(&app, req).await
    }

    pub(crate) async fn get_status(&self) -> api::VerificationStatus {
        let app = build_test_app!(self).await;
        let req = test::TestRequest::post()
            .uri("/status")
            .set_json(serde_json::json!({
                "email": "benjcape@gmail.com"
            }))
            .to_request();

        test::call_and_read_body_json(&app, req).await
    }

    pub(crate) async fn register<T>(&self, secret: &str, data: &T) -> String
    where
        T: Serialize,
    {
        let app = build_test_app!(self).await;
        let req = test::TestRequest::post()
            .uri("/register")
            .set_json(serde_json::json!({
                "email": "benjcape@gmail.com",
                "secret_component": secret,
                "data": data
            }))
            .to_request();

        test::call_and_read_body_json(&app, req).await
    }

    pub(crate) async fn verify_register(&self, otp: &str) {
        let app = build_test_app!(self).await;
        let req = test::TestRequest::post()
            .uri("/register/verify")
            .set_json(serde_json::json!({
                "email": "benjcape@gmail.com",
                "otp": otp
            }))
            .to_request();

        test::call_service(&app, req).await;
    }

    pub(crate) async fn auth(&self) -> String {
        let app = build_test_app!(self).await;
        let req = test::TestRequest::post()
            .uri("/authenticate")
            .set_json(serde_json::json!({
                "email": "benjcape@gmail.com"
            }))
            .to_request();

        test::call_and_read_body_json(&app, req).await
    }

    pub(crate) async fn verify_auth<T>(&self, data: &T) -> String
    where
        T: Serialize,
    {
        let app = build_test_app!(self).await;
        let req = test::TestRequest::post()
            .uri("/authenticate/verify")
            .set_json(serde_json::json!({
                "email": "benjcape@gmail.com",
                "data": data
            }))
            .to_request();

        println!("{:?}", req);

        test::call_and_read_body_json(&app, req).await
    }
}

pub(crate) use build_test_app;
use serde::Serialize;
