use async_trait::async_trait;
use derive::*;
use sqlx::PgPool;

use super::{base::BaseAuthenticator, AuthenticatorServer, VerificationStatus};
use actix_web::HttpResponse;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct QuestionAnswer {
    question: String,
    answer: String,
}

#[PassRequest]
pub struct QA {
    data: QuestionAnswer,
}

#[PassServer(QA)]
pub struct QAAuthenticator {}


#[async_trait]
impl AuthenticatorServer for QAAuthenticator {
    type Data = QuestionAnswer;
    async fn register_verify(
        &self,
        email: &str,
        secret_component: &str,
        data: serde_json::Value,
    ) -> Option<HttpResponse> {
        let qa = data;

        // We should probably hash this...
        let json_data = match serde_json::to_value(qa) {
            Err(_) => {
                return Some(
                    actix_web::HttpResponseBuilder::new(StatusCode::NOT_ACCEPTABLE)
                        .json("Invalid JSON"),
                )
            }
            Ok(r) => r,
        };

        sqlx::query!("INSERT INTO authenticated (email, secret_component, status, data) VALUES ($1, $2, $3, $4) ON CONFLICT (email) DO UPDATE SET secret_component = EXCLUDED.secret_component, data = EXCLUDED.data;",
                        email,
                        secret_component,
                        VerificationStatus::Verified as VerificationStatus,
                        json_data
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
        // // We should probably hash this...
        let qa = data;

        sqlx::query!(
            "UPDATE authenticated SET status=$3 WHERE email=$1 AND (status=$2 OR status=$3) AND data->>'question' = $4 AND data->>'answer' = $5;",
            email,
            VerificationStatus::Verified as VerificationStatus,
            VerificationStatus::RequestAuth as VerificationStatus,
            qa.question,
            qa.answer
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
