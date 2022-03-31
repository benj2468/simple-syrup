use async_trait::async_trait;
use derive::*;
use sqlx::PgPool;

use super::{base::BaseAuthenticator, AuthenticatorServer, VerificationStatus};
use actix_web::HttpResponse;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct Pass {
    pub password: String
}

#[PassRequest]
pub struct Password {
    data: Pass,
}

#[PassServer(Password)]
pub struct PasswordAuthenticator {}


#[async_trait]
impl AuthenticatorServer for PasswordAuthenticator {
    type Data = Pass;
    async fn register_verify(
        &self,
        email: &str,
        secret_component: &str,
        data: serde_json::Value,
    ) -> Option<HttpResponse> {
        sqlx::query!("INSERT INTO authenticated (email, secret_component, status, data) VALUES ($1, $2, $3, $4) ON CONFLICT (email) DO UPDATE SET secret_component = EXCLUDED.secret_component, data = EXCLUDED.data;",
                        email,
                        secret_component,
                        VerificationStatus::Verified as VerificationStatus,
                        data
                    )
                    .execute(&self.base.pool)
                    .await
                    .map_err(|e| actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()))
                    .err()
    }

    async fn verify_authentication(
        &self,
        email: &str,
        data: &Self::Data,
    ) -> Option<HttpResponse> {

        sqlx::query!(
            "UPDATE authenticated SET status=$3 WHERE email=$1 AND (status=$2 OR status=$3) AND data->>'password' =$4;",
            email,
            VerificationStatus::Verified as VerificationStatus,
            VerificationStatus::RequestAuth as VerificationStatus,
            data.password
        )
        .execute(&self.base.pool)
        .await
        .map_err(|e| 
            actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST)
                .json(e.to_string()),
        )
        .err()
    }
}

pub fn server_builder(pool: PgPool) -> PasswordAuthenticator {
    PasswordAuthenticator {
        base: BaseAuthenticator::new(pool),
    }
}
