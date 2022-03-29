use std::fmt::Debug;

use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// Need to figure out how to get this working...
#[derive(sqlx::Type, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerificationStatus {
    Requested,
    Verified,
    RequestAuth,
}

#[async_trait]
pub trait AuthenticatorServer {
    type VerifyData;

    /*
    The Options here are a reason for failure. If there is no reason for failure, that means we did not fail.
    - None => Good! :)
    - Some(error) => Bad, return the error
    */

    /// Verifies the registration.
    ///
    /// This includes inserting any data into the table that is received upon verifying registration. This only happens for some verification servers.
    /// This also must save the secret component. This happens for ALL authentication servers.
    ///
    /// i.e. With QA verification, this includes saving the QA that the user specified.
    /// i.e. With Email verification, this only includes saving the secret_component
    async fn register_verify(
        &self,
        email: &str,
        secret_component: &str,
        data: &Self::VerifyData,
    ) -> Option<HttpResponse>;

    /// Authenticates the user upon request.
    ///
    /// This happens AFTER the user is verified. Verification happens through the same process for everyone - verifying their email
    ///
    /// For other servers, such as QA, authentication is redundant.
    /// Authentication only is required when the server must send data to the user to verify identity, such as OTP.
    async fn authenticate(&self, _email: &str) -> Option<HttpResponse> {
        None
    }

    /// Verify that the user is who they say they are.
    ///
    /// This might take the OTP and confirm it. Or it might take some other data the user sends and confirm it some other way.
    ///
    /// Any API call to a 3rd party would happen here (faceID, etc.)
    async fn verify_authentication(
        &self,
        email: &str,
        data: &Self::VerifyData,
    ) -> Option<HttpResponse>;
}

pub trait ServerRequest {
    fn get_email(&self) -> &String;
    fn get_secret_component(&self) -> Option<&String>;
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
