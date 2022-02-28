use super::DeriveData;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub(crate) fn derive_register(input: &DeriveData) -> TokenStream2 {
    let DeriveData {
        ident, req_ident, ..
    } = input;
    quote! {
        #[actix_web::post("/register")]
        pub async fn register(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let data = request.get_data();
            let secret_component = request.get_secret_component();

            let sec = match secret_component {
                None => return actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json("Must provide secret component"),
                Some(r) => r
            };

            if let Err(e) = authenticator.authenticate(data).await { return e }

            let json_data = match serde_json::to_value(data) {
                Err(e) => return actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()),
                Ok(r) => r
            };

            sqlx::query!(
                "INSERT INTO authenticated (data, secret_component, status) VALUES ($1, $2, $3) ON CONFLICT (data) DO UPDATE SET secret_component = EXCLUDED.secret_component;",
                json_data,
                &sec,
                VerificationStatus::Requested as VerificationStatus
            )
            .execute(&authenticator.pool)
            .await
            .map(|_| actix_web::HttpResponseBuilder::new(StatusCode::OK).finish())
            .unwrap_or_else(|e| actix_web::HttpResponseBuilder::new(StatusCode::FORBIDDEN).json(e.to_string()))
        }
    }
}

pub(crate) fn derive_verify_register(input: &DeriveData) -> TokenStream2 {
    let DeriveData {
        ident, req_ident, ..
    } = input;
    quote! {
        #[actix_web::post("/register/verify")]
        pub async fn register_check(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let data = request.get_data();

            let json_data = match serde_json::to_value(data) {
                Err(e) => return actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()),
                Ok(r) => r
            };

            if authenticator.verify_authentication(&request).await {
                sqlx::query!("UPDATE authenticated SET status=$2 WHERE data=$1 AND status=$3;",
                        json_data,
                        VerificationStatus::Verified as VerificationStatus,
                        VerificationStatus::Requested as VerificationStatus
                    )
                        .execute(&authenticator.pool).await
                        .map(|_| actix_web::HttpResponseBuilder::new(StatusCode::OK).finish())
                        .unwrap_or_else(|e| actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()))
            } else {
                actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).finish()
            }
        }
    }
}

pub(crate) fn derive_authenticate(input: &DeriveData) -> TokenStream2 {
    let DeriveData {
        ident, req_ident, ..
    } = input;
    quote! {
        #[actix_web::post("/authenticate")]
        pub async fn auth(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let data = request.get_data();

            let json_data = match serde_json::to_value(data) {
                Err(e) => return actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()),
                Ok(r) => r
            };

            if let Err(e) = authenticator.authenticate(data).await { return e }

            sqlx::query!(
                "UPDATE authenticated SET status=$2 WHERE data=$1 AND status=$3 OR status=$4;",
                json_data,
                VerificationStatus::RequestAuth as VerificationStatus,
                VerificationStatus::Verified as VerificationStatus,
                VerificationStatus::RequestAuth as VerificationStatus
            )

                .execute(&authenticator.pool)
                .await
                .map(|_| actix_web::HttpResponseBuilder::new(StatusCode::OK).finish())
                .unwrap_or_else(|e| actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()))
        }
    }
}

pub(crate) fn derive_verify_authentication(input: &DeriveData) -> TokenStream2 {
    let DeriveData {
        ident, req_ident, ..
    } = input;
    quote! {
        #[actix_web::post("/authenticate/verify")]
        pub async fn auth_check(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let data = request.get_data();

            let json_data = match serde_json::to_value(data) {
                Err(e) => return actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()),
                Ok(r) => r
            };

            if authenticator.verify_authentication(&request).await {
                sqlx::query!("UPDATE authenticated SET status=$2 WHERE data=$1 AND status=$3 RETURNING secret_component;",
                        json_data,
                        VerificationStatus::Verified as VerificationStatus,
                        VerificationStatus::RequestAuth as VerificationStatus
                    )
                        .fetch_one(&authenticator.pool).await
                        .map(|rec| rec.secret_component)
                        .map(|s: Option<String>| actix_web::HttpResponseBuilder::new(StatusCode::OK).json(s))
                        .unwrap_or_else(|e| actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()))
            } else {
                actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).finish()
            }
        }
    }
}

pub(crate) fn derive_status(input: &DeriveData) -> TokenStream2 {
    let DeriveData {
        ident, req_ident, ..
    } = input;
    quote! {
        #[actix_web::post("/status")]
        pub async fn status_check(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            use sqlx::Row;

            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let data = request.get_data();

            let json_data = match serde_json::to_value(data) {
                Err(e) => return actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()),
                Ok(r) => r
            };

            sqlx::query::<sqlx::Postgres>("SELECT status FROM authenticated WHERE data=$1;")
                .bind(json_data)
                .fetch_one(&authenticator.pool)
                .await
                .map(|row| row.try_get("status").unwrap())
                .map(|s: VerificationStatus| actix_web::HttpResponseBuilder::new(StatusCode::OK).json(s))
                .unwrap_or_else(|e| actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()))
        }
    }
}

pub(crate) fn derive_meta(input: &DeriveData) -> TokenStream2 {
    let server_ty = input.ident.to_string();

    quote! {
        #[actix_web::get("/ty")]
        pub async fn server_ty(req: actix_web::HttpRequest) -> impl actix_web::Responder {
            actix_web::HttpResponseBuilder::new(StatusCode::OK).json(#server_ty)
        }
    }
}

pub(crate) fn derive(input: DeriveData) -> TokenStream2 {
    let register = derive_register(&input);
    let ver_register = derive_verify_register(&input);
    let auth = derive_authenticate(&input);
    let ver_auth = derive_verify_authentication(&input);
    let meta_data = derive_meta(&input);
    let status = derive_status(&input);

    let re_struct: TokenStream2 = input.into();

    quote! {

        #re_struct

        #meta_data

        #register

        #ver_register

        #auth

        #ver_auth

        #status
    }
}
