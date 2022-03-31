use async_trait::async_trait;
use derive::*;
use sqlx::PgPool;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::{base::BaseAuthenticator, AuthenticatorServer, VerificationStatus};
use actix_web::HttpResponse;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
pub struct QuestionAnswer {
    question: String,
    answer: String,
}
#[PassServer(QuestionAnswer, Hashed)]
pub struct QAAuthenticator {}


#[async_trait]
impl AuthenticatorServer for QAAuthenticator {
    type Data = QuestionAnswer;

    async fn verify_authentication(
        &self,
        email: &str,
        data: &Self::Data,
    ) -> Option<HttpResponse> {

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let qa = serde_json::to_value(hasher.finish()).unwrap();

        sqlx::query!(
            "UPDATE authenticated SET status=$3 WHERE email=$1 AND (status=$2 OR status=$3) AND data = $4;",
            email,
            VerificationStatus::Verified as VerificationStatus,
            VerificationStatus::RequestAuth as VerificationStatus,
            qa
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

pub fn server_builder(pool: PgPool) -> QAAuthenticator {
    QAAuthenticator {
        base: BaseAuthenticator::new(pool),
    }
}
