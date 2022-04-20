use crate::DataStorage;

use super::{DeriveData, Idents};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub(crate) fn derive_register(input: &DeriveData) -> TokenStream2 {
    let DeriveData { ident, request, .. } = input;

    let req_ident = &request.idents.request_register;
    let data_type = &request.idents.base;

    let data = match request.data_storage_ty {
        DataStorage::Stored => quote! {
            &request.data
        },
        DataStorage::Hashed => quote! {
            &{
                let mut hasher = DefaultHasher::new();
                request.data.hash(&mut hasher);
                hasher.finish()
            }
        },
        DataStorage::Ignored => quote! {
            &#data_type::default()
        },
    };

    quote! {
        #[actix_web::post("/register")]
        pub async fn register(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let email = &request.email;
            let secret_component = &request.secret_component;


            authenticator.base.prepare(email, secret_component, #data)
                .await
        }
    }
}

pub(crate) fn derive_register_verify(input: &DeriveData) -> TokenStream2 {
    let DeriveData { ident, request, .. } = input;

    let req_ident = &request.idents.verify_register;

    quote! {
        #[actix_web::post("/register/verify")]
        pub async fn register_check(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let otp = &request.otp;
            let email = &request.email;

            match authenticator.base.get_prepared(email)
                .await
                .into_iter()
                .find(|(id, _, _)| BaseAuthenticator::verify(&id, otp))
                {
                    Some((_, sec, data)) => {
                        authenticator.base.verify_register(email, &sec, data)
                        .await
                        .unwrap_or_else(|| actix_web::HttpResponseBuilder::new(StatusCode::OK).finish())
                    },
                    None => return actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).finish()
                }
        }
    }
}

pub(crate) fn derive_authenticate(input: &DeriveData) -> TokenStream2 {
    let DeriveData { ident, request, .. } = input;

    let req_ident = &request.idents.request_auth;

    quote! {
            #[actix_web::post("/authenticate")]
            pub async fn auth(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
                use crate::api::TestDefault;
                let authenticator = req.app_data::<#ident>().unwrap();

                let request = request.0;

                let email = &request.email;

                let auth_data = authenticator.authenticate(email).await;
                if (!cfg!(test)) {
                    if let Some(e) = auth_data { return e };
                };

                sqlx::query!(
                    "UPDATE authenticated SET status=$2 WHERE email=$1 AND status=$3 OR status=$4 RETURNING id;",
                    BaseAuthenticator::hash(email),
                    VerificationStatus::RequestAuth as VerificationStatus,
                    VerificationStatus::Verified as VerificationStatus,
                    VerificationStatus::RequestAuth as VerificationStatus
                )
                    .fetch_one(&authenticator.base.pool)
                    .await
                    .map(|_| actix_web::HttpResponseBuilder::new(StatusCode::OK).finish())
    <<<<<<< HEAD
                    .or_test_default(auth_data.unwrap())
    =======
                    .or_test_default_else(|| auth_data.unwrap())
    >>>>>>> origin/main
                    .unwrap_or_else(|e| actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).json(e.to_string()))
            }
        }
}

pub(crate) fn derive_verify_authentication(input: &DeriveData) -> TokenStream2 {
    let DeriveData { ident, request, .. } = input;

    let req_ident = &request.idents.verify_auth;

    quote! {
        #[actix_web::post("/authenticate/verify")]
        pub async fn auth_check(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let email = &request.email;

            match authenticator.verify_authentication(email, &request.data).await {
                Some(err) => err,
                None => {
                    sqlx::query!("UPDATE authenticated SET status=$2 WHERE email=$1 AND status=$3 RETURNING secret_component;",
                        BaseAuthenticator::hash(email),
                        VerificationStatus::Verified as VerificationStatus,
                        VerificationStatus::RequestAuth as VerificationStatus,
                    )
                        .fetch_one(&authenticator.base.pool)
                        .await
                        .map(|rec| rec.secret_component)
                        .map(|s: Option<String>| actix_web::HttpResponseBuilder::new(StatusCode::OK).json(s))
                        .unwrap_or_else(|e| actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).json(e.to_string()))
                }
            }
        }
    }
}

pub(crate) fn derive_status(input: &DeriveData) -> TokenStream2 {
    let DeriveData { ident, request, .. } = input;

    let req_ident = &request.idents.request_auth;

    quote! {
        #[actix_web::post("/status")]
        pub async fn status_check(req: actix_web::HttpRequest, request: actix_web::web::Json<#req_ident>) -> impl actix_web::Responder {
            use sqlx::Row;

            let authenticator = req.app_data::<#ident>().unwrap();

            let request = request.0;

            let email = request.email;

            sqlx::query::<sqlx::Postgres>("SELECT status FROM authenticated WHERE email=$1;")
                .bind(BaseAuthenticator::hash(&email))
                .fetch_one(&authenticator.base.pool)
                .await
                .map(|row| row.try_get("status").expect("Could not get status field"))
                .map(|s: VerificationStatus| actix_web::HttpResponseBuilder::new(StatusCode::OK).json(s))
                .unwrap_or_else(|e| actix_web::HttpResponseBuilder::new(StatusCode::OK).finish())
        }
    }
}

pub(crate) fn derive_meta(input: &DeriveData) -> TokenStream2 {
    let server_ty = &input.server_ty;

    quote! {
        #[actix_web::get("/ty")]
        pub async fn server_ty(req: actix_web::HttpRequest) -> impl actix_web::Responder {
            actix_web::HttpResponseBuilder::new(StatusCode::OK).json(serde_json::to_value(#server_ty).unwrap())
        }
    }
}

pub(crate) fn derive_req(input: &DeriveData) -> TokenStream2 {
    let DeriveData { request, .. } = input;

    let Idents {
        request_auth,
        request_register,
        verify_register,
        verify_auth,
        base,
    } = &request.idents;

    let request_register_struct = match request.data_storage_ty {
        DataStorage::Stored | DataStorage::Hashed => quote! {
            #[derive(Debug, Deserialize, Serialize)]
            pub struct #request_register {
                email: String,
                secret_component: String,
                data: #base
            }
        },
        DataStorage::Ignored => quote! {
            #[derive(Debug, Deserialize, Serialize)]
            pub struct #request_register {
                email: String,
                secret_component: String,
            }
        },
    };

    quote! {

        #request_register_struct

        #[derive(Debug, Deserialize, Serialize)]
        pub struct #verify_register {
            email: String,
            otp: String,
        }

        #[derive(Debug, Deserialize, Serialize)]
        pub struct #request_auth {
            email: String,
        }


        #[derive(Debug, Deserialize, Serialize)]
        pub struct #verify_auth {
            email: String,
            data: #base
        }

    }
}

impl From<DeriveData> for TokenStream2 {
    fn from(data: DeriveData) -> Self {
        let DeriveData {
            ident,
            fields,
            sub_attrs,
            ..
        } = data;
        quote! {
            #(#sub_attrs)*
            pub struct #ident {
                pub base: super::base::BaseAuthenticator,
                #(#fields,)*
            }
        }
    }
}

pub(crate) fn derive(input: &DeriveData) -> TokenStream2 {
    let register = derive_register(input);
    let ver_register = derive_register_verify(input);
    let auth = derive_authenticate(input);
    let ver_auth = derive_verify_authentication(input);
    let meta_data = derive_meta(input);
    let status = derive_status(input);

    let request_structures = derive_req(input);

    quote! {

        #request_structures

        #meta_data

        #register

        #ver_register

        #auth

        #ver_auth

        #status


    }
}
