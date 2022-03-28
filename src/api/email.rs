use async_trait::async_trait;
use derive::*;
use sqlx::PgPool;

use super::{base::BaseAuthenticator, AuthenticatorServer, ServerRequest, VerificationStatus};
use actix_web::HttpResponse;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

#[PassRequest]
pub struct EmailRequest {}
#[PassServer(EmailRequest)]
pub struct EmailAuthenticator {}

#[async_trait]
impl AuthenticatorServer for EmailAuthenticator {
    type VerifyData = EmailRequest;

    async fn authenticate(&self, email: &str) -> Option<HttpResponse> {
        let id = match self.base.get_authenticated_id(email).await {
            Some(i) => i,
            None => {
                return Some(actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).finish())
            }
        };

        self.base.register(&id.to_string(), email).await
    }
    async fn register_verify(
        &self,
        email: &str,
        secret_component: &str,
        _data: &Self::VerifyData,
    ) -> Option<HttpResponse> {
        sqlx::query!("INSERT INTO authenticated (email, secret_component, status) VALUES ($1, $2, $3) ON CONFLICT (email) DO UPDATE SET secret_component = EXCLUDED.secret_component;",
                        email,
                        secret_component,
                        VerificationStatus::Verified as VerificationStatus,
                    )
                    .execute(&self.base.pool)
                    .await
                    .map_err(|e| actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()))
                    .err()
    }
    async fn verify_authentication(
        &self,
        email: &str,
        data: &Self::VerifyData,
    ) -> Option<HttpResponse> {
        let id = match self.base.get_authenticated_id(email).await {
            Some(i) => i,
            None => {
                return Some(actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).finish())
            }
        };

        (!BaseAuthenticator::verify(&id.to_string(), &data.otp))
            .then_some(actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).finish())
    }
}

pub fn server_builder(pool: PgPool) -> EmailAuthenticator {
    EmailAuthenticator {
        base: BaseAuthenticator::new(pool),
    }
}
