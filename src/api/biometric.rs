use async_trait::async_trait;
use derive::*;
use sqlx::PgPool;

use super::{base::BaseAuthenticator, AuthenticatorServer, VerificationStatus};
use actix_web::{HttpResponse};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};


#[PassServer(
    data(String), 
    store(Stored), 
    ty(crate::config::ServerType::Biometric)
)]
pub struct BiometricAuthenticator {
    pub(crate) api_url: String,
}

impl BiometricAuthenticator {
    fn request_auth_url(&self) -> String {
        format!("{}/requestAuth", self.api_url)
    }
    fn status_url(&self) -> String {
        format!("{}/status", self.api_url)
    }
}

#[async_trait]
impl AuthenticatorServer for BiometricAuthenticator {
    type Data = String;

    async fn authenticate(&self, email: &str) -> Option<HttpResponse> {

        let client = reqwest::Client::new();

        let device_id: Option<String> = sqlx::query!("SELECT data from authenticated WHERE email=$1;", BaseAuthenticator::hash(email))
            .fetch_one(&self.base.pool)
            .await
            .ok()
            .and_then(|record| record.data)
            .and_then(|data| serde_json::from_value(data).ok());

        if let Some(id) = device_id {
            client.post(&self.request_auth_url())
                .json(&serde_json::json!({ "deviceId": id }))
                .send()
                .await
                .err()
                .map(|_| HttpResponse::new(StatusCode::BAD_REQUEST))
        } else {
            Some(actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json("You were not authenticated."))
        }
    }

    async fn verify_authentication(
        &self,
        email: &str,
        _data: &Self::Data,
    ) -> Option<HttpResponse> {

        let device_id: Option<String> = sqlx::query!("SELECT data from authenticated WHERE email=$1;", BaseAuthenticator::hash(email))
            .fetch_one(&self.base.pool)
            .await
            .ok()
            .and_then(|record| record.data)
            .and_then(|data| serde_json::from_value(data).ok());

        if let Some(id) = device_id {
            let client = reqwest::Client::new();
            let res = client.get(&self.status_url())
                .json(&serde_json::json!({ "deviceId": id }))
                .send()
                .await;

            if let Ok(res) = res {
                // This should not be an unwrap... but oh well for now
                let status: bool = res.json().await.unwrap();
                if !status {
                    return Some(HttpResponse::new(StatusCode::UNAUTHORIZED));
                }
            }
        } else {
            return Some(actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json("You were not authenticated."))
        }

        sqlx::query!(
            "UPDATE authenticated SET status=$3 WHERE email=$1 AND (status=$2 OR status=$3) RETURNING id;",
            BaseAuthenticator::hash(email),
            VerificationStatus::Verified as VerificationStatus,
            VerificationStatus::RequestAuth as VerificationStatus,
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

pub fn server_builder(pool: PgPool) -> BiometricAuthenticator {
    let api_url = std::env::var("BIOMETRIC_API_URL").expect("need BIOMETRIC_API_URL to test");
    BiometricAuthenticator {
        base: BaseAuthenticator::new(pool),
        api_url
    }
}



