use async_trait::async_trait;
use derive::PassServer;
use sqlx::PgPool;

use super::base::Handlers;
use super::{base::BaseAuthenticator, AuthenticatorServer, VerificationStatus};
use actix_web::HttpResponse;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

#[PassServer(data(String), store(Ignored), ty(crate::config::ServerType::Email))]
pub struct EmailAuthenticator {}

#[async_trait]
impl AuthenticatorServer for EmailAuthenticator {
    type Data = String;

    async fn authenticate(&self, email: &str) -> Option<HttpResponse> {
        let id = match self.base.get_authenticated_id(email).await {
            Some(i) => i,
            None => {
                return Some(actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).finish())
            }
        };
        self.base.register(&id.to_string(), email).await
    }
    async fn verify_authentication(&self, email: &str, data: &Self::Data) -> Option<HttpResponse> {
        let id = match self.base.get_authenticated_id(email).await {
            Some(i) => i,
            None => {
                return Some(actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).finish())
            }
        };

        if BaseAuthenticator::verify(&id.to_string(), data) {
            None
        } else {
            Some(actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).finish())
        }
    }
}

pub fn server_builder(pool: PgPool) -> EmailAuthenticator {
    EmailAuthenticator {
        base: BaseAuthenticator::new(pool),
    }
}
