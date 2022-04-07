use async_trait::async_trait;
use derive::*;
use sqlx::PgPool;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::ServerData;
use super::{base::BaseAuthenticator, AuthenticatorServer, VerificationStatus};
use actix_web::HttpResponse;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Default)]
pub struct Pass {
    pub password: String
}

#[PassServer(
    data(Pass), 
    store(Hashed), 
    ty(crate::config::ServerType::Password)
)]
pub struct PasswordAuthenticator {}

impl ServerData for Pass {
    fn bad_data() -> Self {
        Self {
            password: "Bad data".to_string()
        }
    }
}


#[async_trait]
impl AuthenticatorServer for PasswordAuthenticator {
    type Data = Pass;

    async fn verify_authentication(
        &self,
        email: &str,
        data: &Self::Data,
    ) -> Option<HttpResponse> {

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let data = serde_json::to_value(hasher.finish()).unwrap();

        sqlx::query!(
            "UPDATE authenticated SET status=$3 WHERE email=$1 AND (status=$2 OR status=$3) AND data = $4 RETURNING id;",
            email,
            VerificationStatus::Verified as VerificationStatus,
            VerificationStatus::RequestAuth as VerificationStatus,
            data
        )
        .fetch_one(&self.base.pool)
        .await
        .map_err(|e| 
            actix_web::HttpResponseBuilder::new(StatusCode::UNAUTHORIZED)
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
