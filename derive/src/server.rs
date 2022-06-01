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
            let authenticator = req.app_data::<#ident>().expect("Could not unwrap Authenticator");

            let request = request.0;

            let email = &request.email;
            let secret_component = &request.secret_component;
            #[cfg(feature = "web3")]
            let contract_address = &request.contract_address;


            authenticator.base.prepare(email, secret_component, #data, #[cfg(feature = "web3")] contract_address)
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
            let authenticator = req.app_data::<#ident>().expect("Could not unwrap Authenticator");

            let request = request.0;

            let otp = &request.otp;
            let email = &request.email;

            match authenticator.base.get_prepared(email)
                .await
                .into_iter()
                .find(|(id, _, _, _)| BaseAuthenticator::verify(&id, otp))
                {
                    Some((_, sec, data, addr)) => {
                        authenticator.base.verify_register(email, &sec, data, &addr)
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
            let authenticator = req.app_data::<#ident>().expect("Could not unwrap Authenticator");

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
                .map(|_| {
                    tracing::info!("Requested Authentication");
                    actix_web::HttpResponseBuilder::new(StatusCode::OK).finish()
                })
                .or_test_default_else(|| auth_data.unwrap())
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
            let authenticator = req.app_data::<#ident>().expect("Could not unwrap Authenticator");

            let request = request.0;

            let email = &request.email;

            match authenticator.verify_authentication(email, &request.data).await {
                Some(err) => err,
                None => {
                    let res = if cfg!(feature = "development") {
                        sqlx::query!("UPDATE authenticated SET status=$2 WHERE email=$1 AND (status=$3 OR status=$4) RETURNING secret_component, contract_address;",
                            BaseAuthenticator::hash(email),
                            VerificationStatus::Verified as VerificationStatus,
                            VerificationStatus::RequestAuth as VerificationStatus,
                            VerificationStatus::Verified as VerificationStatus,
                        )
                        .fetch_one(&authenticator.base.pool)
                        .await
                        .map(|rec| rec.secret_component.and_then(|s| rec.contract_address.map(|c| (s, c))))
                    } else {
                        sqlx::query!("UPDATE authenticated SET status=$2 WHERE email=$1 AND status=$3 RETURNING secret_component, contract_address;",
                            BaseAuthenticator::hash(email),
                            VerificationStatus::Verified as VerificationStatus,
                            VerificationStatus::RequestAuth as VerificationStatus,
                        )
                        .fetch_one(&authenticator.base.pool)
                        .await
                        .map(|rec| rec.secret_component.and_then(|s| rec.contract_address.map(|c| (s, c))))
                    };

                    match res {
                        Ok(secret_and_addr) => {
                            let (secret, contract_address) = secret_and_addr.map_or_else(
                                || (None, Default::default()),
                                |c| (Some(c.0), c.1)
                            );

                            #[cfg(not(feature = "web3"))]
                            match Handlers::web2_handler(secret).await {
                                Some(err) => err,
                                None => actix_web::HttpResponseBuilder::new(StatusCode::OK).finish()
                            }

                            #[cfg(feature = "web3")]
                            if (request.useWeb3) {
                                match Handlers::web3_handler(&authenticator.base.web3_config, secret, &contract_address, &request.destination_address).await {
                                    Some(res) => res,
                                    None => actix_web::HttpResponseBuilder::new(StatusCode::OK).finish()
                                }
                            } else {
                                match Handlers::web2_handler(secret).await {
                                    Some(res) => res,
                                    None => actix_web::HttpResponseBuilder::new(StatusCode::OK).finish()
                                }
                            }
                        },
                        Err(e) => actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).json(e.to_string())
                    }
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

            let authenticator = req.app_data::<#ident>().expect("Could not unwrap Authenticator");

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
                #[cfg(feature = "web3")]
                contract_address: String,
                data: #base
            }
        },
        DataStorage::Ignored => quote! {
            #[derive(Debug, Deserialize, Serialize)]
            pub struct #request_register {
                email: String,
                secret_component: String,
                #[cfg(feature = "web3")]
                contract_address: String,
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
            data: #base,
            #[cfg(feature = "web3")]
            destination_address: String,
            #[cfg(feature = "web3")]
            useWeb3: bool,
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
