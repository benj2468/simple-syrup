use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::SystemTime,
};

#[cfg(feature = "web3")]
use std::str::FromStr;

use actix_web::{HttpResponse, HttpResponseBuilder};
use hyper::StatusCode;
use sqlx::{types::Uuid, PgPool};
use totp_rs::TOTP;
#[cfg(feature = "web3")]
use web3::{transports::WebSocket, Web3};

use crate::api::VerificationStatus;

#[cfg(feature = "web3")]
pub struct Web3Config {
    account: web3::types::Address,
    websocket_key: String,
}

pub struct BaseAuthenticator {
    #[cfg(not(test))]
    pub sg_client: sendgrid::SGClient,
    pub pool: sqlx::Pool<sqlx::Postgres>,
    #[cfg(not(test))]
    #[cfg(feature = "web3")]
    pub web3_config: Web3Config,
}

impl BaseAuthenticator {
    pub fn new(pool: PgPool) -> Self {
        #[cfg(not(test))]
        let my_secret_key = std::env::var("SENDGRID_KEY").expect("need SENDGRID_KEY to test");

        #[cfg(not(test))]
        #[cfg(feature = "web3")]
        let web3_config = {
            let my_address = web3::types::Address::from_str(
                &std::env::var("ETH_ADDRESS").expect("Must provide an address for this server!"),
            )
            .expect("Must provide a VIABLE address for this server");

            let websocket_key =
                std::env::var("INFURA_RINKEBY").expect("Must provide an INFURA_RINKEBY");

            Web3Config {
                account: my_address,
                websocket_key,
            }
        };

        Self {
            #[cfg(not(test))]
            sg_client: sendgrid::SGClient::new(my_secret_key),
            pool,
            #[cfg(not(test))]
            #[cfg(feature = "web3")]
            web3_config,
        }
    }

    pub fn hash(s: &str) -> String {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish().to_string()
    }

    pub async fn prepare<T>(&self, email: &str, sec: &str, data: &T) -> HttpResponse
    where
        T: serde::Serialize,
    {
        let res = sqlx::query!(
            "INSERT INTO prepare (email, secret_component, data) VALUES ($1, $2, $3) RETURNING id",
            Self::hash(email),
            &sec,
            serde_json::to_value(data).expect("Could not serialize data"),
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
                        tracing::info!("Requested Verification");
                        actix_web::HttpResponseBuilder::new(StatusCode::OK).finish()
                    }),
                None => actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).finish(),
            },
            Err(e) => {
                actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string())
            }
        }
    }
    #[cfg(not(test))]
    async fn send_email(
        &self,
        email: &str,
        otp: &str,
    ) -> sendgrid::SendgridResult<reqwest::Response> {
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

    #[cfg(test)]
    async fn send_email(&self, _email: &str, _otp: &str) -> Result<(), String> {
        Ok(())
    }

    pub fn verify(id: &str, otp: &str) -> bool {
        let totp = TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, id);
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        totp.check(otp, time)
    }

    pub async fn register(&self, id: &str, email: &str) -> Option<actix_web::HttpResponse> {
        let totp = TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, id);
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let otp = totp.generate(time);

        if cfg!(test) {
            Some(actix_web::HttpResponseBuilder::new(StatusCode::OK).json(otp))
        } else {
            self.send_email(email, &otp)
                .await
                .map_err(|e| {
                    actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string())
                })
                .err()
        }
    }

    pub async fn verify_register(
        &self,
        email: &str,
        secret_component: &str,
        data: serde_json::Value,
    ) -> Option<HttpResponse> {
        sqlx::query!("INSERT INTO authenticated (email, secret_component, status, data) VALUES ($1, $2, $3, $4) ON CONFLICT (email) DO UPDATE SET secret_component = EXCLUDED.secret_component, data = EXCLUDED.data;",
                        Self::hash(email),
                        secret_component,
                        VerificationStatus::Verified as VerificationStatus,
                        data
                    )
                    .execute(&self.pool)
                    .await
                    .map_err(|e| actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string()))
                    .err()
                    .map(|e| {
                        tracing::info!("Successful Verification");
                        e
                    })
    }

    pub async fn get_prepared(&self, email: &str) -> Vec<(String, String, serde_json::Value)> {
        sqlx::query!(
            "SELECT id, secret_component, data from prepare WHERE email=$1",
            Self::hash(email)
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|_| vec![])
        .iter()
        .map(|rec| {
            (
                rec.id as Option<Uuid>,
                rec.secret_component.clone() as Option<String>,
                rec.data.clone() as Option<serde_json::Value>,
            )
        })
        .filter(|(a, b, c)| a.and(b.as_ref()).and(c.as_ref()).is_some())
        .map(|(id, sec, data)| (id.unwrap().to_string(), sec.unwrap(), data.unwrap()))
        .collect()
    }

    pub async fn get_authenticated_id(&self, email: &str) -> Option<Uuid> {
        sqlx::query!(
            "SELECT id from authenticated WHERE email=$1",
            Self::hash(email)
        )
        .fetch_one(&self.pool)
        .await
        .ok()
        .and_then(|rec| rec.id)
    }
}

pub(crate) struct Handlers;

impl Handlers {
    #[cfg(feature = "web3")]
    async fn _build_web3_client(websocket_key: &str) -> Result<Web3<WebSocket>, HttpResponse> {
        let ws = web3::transports::WebSocket::new(websocket_key)
            .await
            .map_err(|e| {
                actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string())
            })?;

        Ok(web3::Web3::new(ws))
    }

    pub(crate) async fn web2_handler(secret_component: Option<String>) -> Option<HttpResponse> {
        tracing::info!(ty = "web2", "Successful authentication");
        Some(HttpResponseBuilder::new(StatusCode::OK).json(secret_component))
    }

    #[cfg(feature = "web3")]
    pub(crate) async fn web3_handler(
        web3_config: Web3Config,
        secret_component: Option<String>,
        contract_address: String,
    ) -> Option<HttpResponse> {
        let Web3Config {
            account,
            websocket_key,
        } = web3_config;

        let web3 = match Self::_build_web3_client(&websocket_key).await {
            Ok(web3) => web3,
            Err(e) => return Some(e),
        };

        let contract = web3::contract::Contract::from_json(
            web3.eth(),
            web3::types::Address::from_str(&contract_address).unwrap(),
            include_bytes!("../../contract/abi.json"),
        );

        if contract.is_err() {
            return Some(
                HttpResponseBuilder::new(StatusCode::BAD_REQUEST)
                    .json(contract.unwrap_err().to_string()),
            );
        }

        let contract = contract.unwrap();

        let res = contract
            .call(
                "contribute",
                (secret_component.unwrap_or_default(),),
                account,
                web3::contract::Options::default(),
            )
            .await;

        match res {
            Ok(_) => None,
            Err(e) => Some(HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string())),
        }

        tracing::info!(ty = "web3", "Successful authentication");
    }
}
