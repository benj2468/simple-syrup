use super::DeriveData;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub(crate) fn derive_register(input: &DeriveData) -> TokenStream2 {
    let DeriveData { ident, attrs, .. } = input;

    let req_ident = attrs.get(0).expect("Must provide a Request Data Type!");

    quote! {
        #[actix_web::post("/register")]
        pub async fn register(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let email = request.get_email();
            let secret_component = request.get_secret_component();

            let sec = match secret_component {
                None => return actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json("Must provide secret component"),
                Some(r) => r
            };

            authenticator.base.prepare(&email, &sec)
                .await
        }
    }
}

pub(crate) fn derive_register_verify(input: &DeriveData) -> TokenStream2 {
    let DeriveData { ident, attrs, .. } = input;

    let req_ident = attrs.get(0).expect("Must provide a Request Data Type!");

    quote! {
        #[actix_web::post("/register/verify")]
        pub async fn register_check(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;
            let otp = &request.otp;

            let email = request.get_email();

            match authenticator.base.get_prepared(email)
                .await
                .into_iter()
                .find(|(id, _)| {
                    println!("{:?} {:?}", id, otp);
                    BaseAuthenticator::verify(&id, otp)
                })
                {
                    Some((_, sec)) => authenticator.register_verify(&email, &sec, &request)
                        .await
                        .unwrap_or_else(|| actix_web::HttpResponseBuilder::new(StatusCode::OK).finish()),
                    None => return actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).finish()
                }
        }
    }
}

pub(crate) fn derive_authenticate(input: &DeriveData) -> TokenStream2 {
    let DeriveData { ident, attrs, .. } = input;

    let req_ident = attrs.get(0).expect("Must provide a Request Data Type!");

    quote! {
        #[actix_web::post("/authenticate")]
        pub async fn auth(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let email = request.get_email();

            if let Some(e) = authenticator.authenticate(email).await { return e }

            sqlx::query!(
                "UPDATE authenticated SET status=$2 WHERE email=$1 AND status=$3 OR status=$4;",
                email,
                VerificationStatus::RequestAuth as VerificationStatus,
                VerificationStatus::Verified as VerificationStatus,
                VerificationStatus::RequestAuth as VerificationStatus
            )
                .execute(&authenticator.base.pool)
                .await
                .map(|_| actix_web::HttpResponseBuilder::new(StatusCode::OK).finish())
                .unwrap_or_else(|e| actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()))
        }
    }
}

pub(crate) fn derive_verify_authentication(input: &DeriveData) -> TokenStream2 {
    let DeriveData { ident, attrs, .. } = input;

    let req_ident = attrs.get(0).expect("Must provide a Request Data Type!");

    quote! {
        #[actix_web::post("/authenticate/verify")]
        pub async fn auth_check(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let email = request.get_email();

            match authenticator.verify_authentication(&email, &request).await {
                Some(err) => err,
                None => {
                    sqlx::query!("UPDATE authenticated SET status=$2 WHERE email=$1 AND status=$3 RETURNING secret_component;",
                        email,
                        VerificationStatus::Verified as VerificationStatus,
                        VerificationStatus::RequestAuth as VerificationStatus,
                    )
                        .fetch_one(&authenticator.base.pool)
                        .await
                        .map(|rec| rec.secret_component)
                        .map(|s: Option<String>| actix_web::HttpResponseBuilder::new(StatusCode::OK).json(s))
                        .unwrap_or_else(|e| actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()))
                }
            }
        }
    }
}

pub(crate) fn derive_status(input: &DeriveData) -> TokenStream2 {
    let DeriveData { ident, attrs, .. } = input;

    let req_ident = attrs.get(0).expect("Must provide a Request Data Type!");

    quote! {
        #[actix_web::post("/status")]
        pub async fn status_check(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            use sqlx::Row;

            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let email = request.get_email();

            sqlx::query::<sqlx::Postgres>("SELECT status FROM authenticated WHERE email=$1;")
                .bind(email)
                .fetch_one(&authenticator.base.pool)
                .await
                .map(|row| row.try_get("status").unwrap())
                .map(|s: VerificationStatus| actix_web::HttpResponseBuilder::new(StatusCode::OK).json(s))
                .unwrap_or_else(|e| actix_web::HttpResponseBuilder::new(StatusCode::OK).finish())
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

pub(crate) fn derive_req(input: DeriveData) -> TokenStream2 {
    let DeriveData { fields, ident, .. } = input;

    quote! {
        #[derive(Debug, Deserialize, Serialize)]
        pub struct #ident {
            email: String,
            otp: Option<String>,
            secret_component: Option<String>,
            #(#fields,)*
        }

        impl ServerRequest for #ident {
            fn get_email(&self) -> &String {
                &self.email
            }
            fn get_secret_component(&self) -> Option<&String> {
                self.secret_component.as_ref()
            }
        }
    }
}

pub(crate) fn derive(input: DeriveData) -> TokenStream2 {
    let register = derive_register(&input);
    let ver_register = derive_register_verify(&input);
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
