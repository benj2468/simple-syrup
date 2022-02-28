use async_trait::async_trait;
use derive::AuthServer;
use sqlx::PgPool;
use std::time::SystemTime;

use super::{AuthenticatorServer, Result, ServerRequest, VerificationStatus};
use actix_web::{HttpResponse, HttpResponseBuilder};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use totp_rs::TOTP;

#[derive(Debug, Deserialize, Serialize)]
pub struct EmailRequest {
    data: String,
    secret_component: Option<String>,
    otp: Option<String>,
}

impl ServerRequest<String> for EmailRequest {
    fn get_data(&self) -> &String {
        &self.data
    }

    fn get_secret_component(&self) -> Option<&String> {
        self.secret_component.as_ref()
    }
}

#[AuthServer(EmailRequest)]
pub struct EmailAuthenticator {
    pub sg_client: sendgrid::SGClient,
}

impl EmailAuthenticator {
    async fn send_email(&self, email: &str, otp: &str) -> Result<(), HttpResponse> {
        let body = format!("Your OTP for CryptoPass: {}", otp);
        let message = sendgrid::Mail::new()
            .add_from("benjcape@gmail.com")
            .add_reply_to("benjcape@gmail.com")
            .add_subject("OTP")
            .add_to(sendgrid::Destination {
                address: email,
                name: "Some name",
            })
            .add_text(&body);

        self.sg_client
            .send(message)
            .await
            .map(|_| ())
            .map_err(|e| HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()))
    }
}

#[async_trait]
impl AuthenticatorServer for EmailAuthenticator {
    type RegisterData = String;
    type VerifyData = EmailRequest;

    async fn authenticate(&self, email: &Self::RegisterData) -> Result<(), HttpResponse> {
        let totp = TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, email);
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let otp = totp.generate(time);

        self.send_email(email, &otp).await
    }
    async fn verify_authentication(&self, data: &Self::VerifyData) -> bool {
        let EmailRequest { data, otp, .. } = data;

        let totp = TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, data);
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        otp.as_ref()
            .map(|token| totp.check(token, time))
            .unwrap_or_default()
    }
}

pub fn server_builder(pool: PgPool) -> EmailAuthenticator {
    let my_secret_key = std::env::var("SENDGRID_KEY").expect("need SENDGRID_KEY to test");

    EmailAuthenticator {
        pool,
        sg_client: sendgrid::SGClient::new(my_secret_key),
    }
}
