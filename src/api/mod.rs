use std::fmt::Debug;

use actix_web::{get, web, HttpRequest, HttpResponse, HttpResponseBuilder, Responder};
use async_trait::async_trait;
use base::Handlers;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

// Need to figure out how to get this working...
#[derive(sqlx::Type, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerificationStatus {
    Requested,
    Verified,
    RequestAuth,
}

pub(crate) trait TestDefault<F, T> {
    fn or_test_default_else(self, default: F) -> Self;
}

impl<F, T> TestDefault<F, T> for Option<T>
where
    F: FnOnce() -> T,
{
    fn or_test_default_else(self, default: F) -> Option<T> {
        if cfg!(test) {
            Some(default())
        } else {
            self
        }
    }
}

impl<F, T, E> TestDefault<F, T> for Result<T, E>
where
    F: FnOnce() -> T,
{
    fn or_test_default_else(self, default: F) -> Result<T, E> {
        if cfg!(test) {
            self.map(|_| default())
        } else {
            self
        }
    }
}

#[async_trait]
pub trait AuthenticatorServer {
    type Data;
    /*
    The Options here are a reason for failure. If there is no reason for failure, that means we did not fail.
    - None => Good! :)
    - Some(error) => Bad, return the error
    */

    /// Authenticates the user upon request.
    ///
    /// This happens AFTER the user is verified. Verification happens through the same process for everyone - verifying their email
    ///
    /// For other servers, such as QA, authentication is redundant.
    /// Authentication only is required when the server must send data to the user to verify identity, such as OTP.
    async fn authenticate(&self, _email: &str) -> Option<HttpResponse> {
        None.or_test_default_else(|| HttpResponseBuilder::new(StatusCode::OK).json(""))
    }

    /// Verify that the user is who they say they are.
    ///
    /// This might take the OTP and confirm it. Or it might take some other data the user sends and confirm it some other way.
    ///
    /// Any API call to a 3rd party would happen here (faceID, etc.)
    async fn verify_authentication(&self, email: &str, data: &Self::Data) -> Option<HttpResponse>;

    /// Handler for secret_component post authentication
    #[cfg(not(feature = "web3"))]
    async fn secret_handler(&self, secret_component: Option<String>) -> Option<HttpResponse> {
        Handlers::web2_handler(secret_component).await
    }

    #[cfg(feature = "web3")]
    async fn secret_handler(
        &self,
        secret_component: Option<String>,
        _addresses: (&str, &str),
    ) -> Option<HttpResponse> {
        Handlers::web2_handler(secret_component).await
    }
}

pub trait ServerData: Default + Serialize {
    fn bad_data() -> Self;
}

impl ServerData for String {
    fn bad_data() -> String {
        "Bad data".to_string()
    }
}

#[get("/")]
pub async fn index(_req: HttpRequest) -> impl Responder {
    web::Json("OK")
}

pub mod base;

#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "qa")]
pub mod qa;

#[cfg(feature = "password")]
pub mod password;

#[cfg(feature = "biometric")]
pub mod biometric;
