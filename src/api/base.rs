use std::time::SystemTime;

use actix_web::HttpResponse;
use hyper::StatusCode;
use sqlx::{types::Uuid, PgPool};
use totp_rs::TOTP;

pub struct BaseAuthenticator {
    pub sg_client: sendgrid::SGClient,
    pub pool: sqlx::Pool<sqlx::Postgres>,
}

impl BaseAuthenticator {
    pub fn new(pool: PgPool) -> Self {
        let my_secret_key = std::env::var("SENDGRID_KEY").expect("need SENDGRID_KEY to test");
        Self {
            sg_client: sendgrid::SGClient::new(my_secret_key),
            pool,
        }
    }
    pub async fn prepare(&self, email: &str, sec: &str) -> HttpResponse {
        let res = sqlx::query!(
            "INSERT INTO prepare (email, secret_component) VALUES ($1, $2) RETURNING id",
            email,
            &sec
        )
        .fetch_one(&self.pool)
        .await
        .map(|rec| rec.id as Option<Uuid>);

        match res {
            Ok(inner) => match inner {
                Some(id) => self
                    .register(&id.to_string(), email)
                    .await
                    .unwrap_or_else(|| {
                        actix_web::HttpResponseBuilder::new(StatusCode::OK).finish()
                    }),
                None => actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).finish(),
            },
            Err(e) => {
                actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string())
            }
        }
    }
    async fn send_email(
        &self,
        email: &str,
        otp: &str,
    ) -> sendgrid::SendgridResult<reqwest::Response> {
        println!("Sending {:?} to {:?}", otp, email);
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

        self.sg_client.send(message).await
    }

    pub fn verify(id: &str, otp: &Option<String>) -> bool {
        let otp = otp.as_ref();

        let totp = TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, id);
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        otp.as_ref()
            .map(|token| totp.check(token, time))
            .unwrap_or_default()
    }

    pub async fn register(&self, id: &str, email: &str) -> Option<actix_web::HttpResponse> {
        let totp = TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, id);
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let otp = totp.generate(time);

        self.send_email(email, &otp)
            .await
            .map_err(|e| {
                actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string())
            })
            .err()
    }

    pub async fn get_prepared(&self, email: &str) -> Vec<(String, String)> {
        sqlx::query!(
            "SELECT id, secret_component from prepare WHERE email=$1",
            email
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|_| vec![])
        .iter()
        .map(|rec| {
            (
                rec.id as Option<Uuid>,
                rec.secret_component.clone() as Option<String>,
            )
        })
        .filter(|(id, _)| id.is_some())
        .map(|(id, sec)| (id.unwrap().to_string(), sec.unwrap()))
        .collect()
    }

    pub async fn get_authenticated_id(&self, email: &str) -> Option<Uuid> {
        sqlx::query!("SELECT id from authenticated WHERE email=$1", email)
            .fetch_one(&self.pool)
            .await
            .ok()
            .and_then(|rec| rec.id)
    }
}
