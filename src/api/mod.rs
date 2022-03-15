use std::fmt::Debug;

use actix_web::{get, web, HttpRequest, HttpResponse, Responder, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// Need to figure out how to get this working...
#[derive(sqlx::Type, Debug, Serialize, Deserialize)]
pub enum VerificationStatus {
    Requested,
    Verified,
    RequestAuth,
}

#[async_trait]
pub trait AuthenticatorServer {
    type RegisterData;
    type VerifyData;
    async fn authenticate(&self, data: &Self::RegisterData) -> Result<(), HttpResponse>;
    async fn verify_authentication(&self, data: &Self::VerifyData) -> bool;
}

pub trait ServerRequest<T> {
    fn get_data(&self) -> &T;
    fn get_secret_component(&self) -> Option<&String>;
}

#[get("/")]
pub async fn index(_req: HttpRequest) -> impl Responder {
    web::Json("OK")
}

#[cfg(feature = "email")]
pub mod email;
