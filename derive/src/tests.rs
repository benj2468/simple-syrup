use super::DeriveData;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub(crate) fn derive(input: &DeriveData) -> TokenStream2 {
    let server_ty = &input.server_ty;

    let data_ty = &input.request.idents.base;

    quote! {
        #[cfg(test)]
        mod generated_tests {
            use super::*;

            #[actix_web::test]
            async fn bad_otp_verify_register() {
                let app = crate::config::Config::test(#server_ty).await;

                let _ = app.register("foobar", &#data_ty::default()).await;

                let app = crate::test::build_test_app!(app).await;

                let res = actix_web::test::call_service(
                    &app,
                    actix_web::test::TestRequest::post()
                        .uri("/register/verify")
                        .set_json(serde_json::json!({
                            "email": "benjcape@gmail.com",
                            "otp": "deadbeef"
                        }))
                        .to_request(),
                )
                .await;

                assert_eq!(res.status(), actix_web::http::StatusCode::UNAUTHORIZED);
            }

            #[actix_web::test]
            async fn no_otp_verify_register() {
                let app = crate::config::Config::test(#server_ty).await;

                let _ = app.register("foobar", &#data_ty::default()).await;

                let app = crate::test::build_test_app!(app).await;

                let res = actix_web::test::call_service(
                    &app,
                    actix_web::test::TestRequest::post()
                        .uri("/register/verify")
                        .set_json(serde_json::json!({
                            "email": "benjcape@gmail.com",
                        }))
                        .to_request(),
                )
                .await;

                assert_eq!(res.status(), actix_web::http::StatusCode::BAD_REQUEST);
            }

            #[actix_web::test]
            async fn bad_verify_register() {
                let app = crate::config::Config::test(#server_ty).await;

                let otp = app.register("foobar", &#data_ty::default()).await;

                let app = crate::test::build_test_app!(app).await;

                let res = actix_web::test::call_service(
                    &app,
                    actix_web::test::TestRequest::post()
                        .uri("/register/verify")
                        .set_json(serde_json::json!({
                            "email": "foobar@foobar.com",
                            "otp": otp
                        }))
                        .to_request(),
                )
                .await;

                assert_eq!(res.status(), actix_web::http::StatusCode::UNAUTHORIZED);
            }

            #[actix_web::test]
            async fn no_verify_register() {
                let app = crate::config::Config::test(#server_ty).await;

                let _ = app.register("foobar", &#data_ty::default()).await;

                let app = crate::test::build_test_app!(app).await;

                let res = actix_web::test::call_service(
                    &app,
                    actix_web::test::TestRequest::post()
                        .uri("/register")
                        .set_json(serde_json::json!({}))
                        .to_request(),
                )
                .await;

                assert_eq!(res.status(), actix_web::http::StatusCode::BAD_REQUEST);
            }

            #[actix_web::test]
            async fn bad_authenticate() {
                let app = crate::config::Config::test(#server_ty).await;

                let secret = "foobar-test-email";

                let otp = app.register(secret, &#data_ty::default()).await;

                app.verify_register(&otp).await;

                let status = app.get_status().await;

                assert_eq!(status, crate::api::VerificationStatus::Verified);

                let app = crate::test::build_test_app!(app).await;
                let req = actix_web::test::TestRequest::post()
                    .uri("/authenticate")
                    .set_json(serde_json::json!({
                        "email": "foobar@foobar.com"
                    }))
                    .to_request();

                let res = actix_web::test::call_service(&app, req).await;

                assert_eq!(res.status(), actix_web::http::StatusCode::UNAUTHORIZED);
            }

            #[actix_web::test]
            async fn no_email_authenticate() {
                let app = crate::config::Config::test(#server_ty).await;

                let secret = "foobar-test-email";

                let otp = app.register(secret, &#data_ty::default()).await;

                app.verify_register(&otp).await;

                let status = app.get_status().await;

                assert_eq!(status, crate::api::VerificationStatus::Verified);

                let app = crate::test::build_test_app!(app).await;
                let req = actix_web::test::TestRequest::post()
                    .uri("/authenticate")
                    .set_json(serde_json::json!({}))
                    .to_request();

                let res = actix_web::test::call_service(&app, req).await;

                assert_eq!(res.status(), actix_web::http::StatusCode::BAD_REQUEST);
            }

            #[actix_web::test]
            async fn no_data_verify_authenticate() {
                let app = crate::config::Config::test(#server_ty).await;

                let secret = "foobar-test-email";

                let otp = app.register(secret, &#data_ty::default()).await;

                app.verify_register(&otp).await;

                let status = app.get_status().await;

                assert_eq!(status, crate::api::VerificationStatus::Verified);

                let _ = app.auth().await;

                let status = app.get_status().await;

                assert_eq!(status, crate::api::VerificationStatus::RequestAuth);

                let app = crate::test::build_test_app!(app).await;

                let req = actix_web::test::TestRequest::post()
                    .uri("/authenticate/verify")
                    .set_json(serde_json::json!({ "email": "benjcape@gmail.com" }))
                    .to_request();

                let res = actix_web::test::call_service(&app, req).await;

                assert_eq!(res.status(), actix_web::http::StatusCode::BAD_REQUEST);
            }

            #[actix_web::test]
            async fn bad_data_verify_authenticate() {
                let app = crate::config::Config::test(#server_ty).await;

                let secret = "foobar-test-email";

                let otp = app.register(secret, &#data_ty::default()).await;

                app.verify_register(&otp).await;

                let status = app.get_status().await;

                assert_eq!(status, crate::api::VerificationStatus::Verified);

                let _ = app.auth().await;

                let status = app.get_status().await;

                assert_eq!(status, crate::api::VerificationStatus::RequestAuth);

                let app = crate::test::build_test_app!(app).await;

                let req = actix_web::test::TestRequest::post()
                    .uri("/authenticate/verify")
                    .set_json(serde_json::json!({
                        "email": "benjcape@gmail.com",
                        "data": #data_ty::bad_data()
                    }))
                    .to_request();

                let res = actix_web::test::call_service(&app, req).await;

                assert_eq!(res.status(), actix_web::http::StatusCode::UNAUTHORIZED);
            }
        }
    }
}
