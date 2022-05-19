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
    #[cfg(feature = "web3")]
    pub web3_config: Web3Config,
}

impl BaseAuthenticator {
    pub fn new(pool: PgPool) -> Self {
        #[cfg(not(test))]
        let my_secret_key = std::env::var("SENDGRID_KEY").expect("need SENDGRID_KEY to test");

        #[cfg(feature = "web3")]
        let web3_config = {
            let my_address = web3::types::Address::from_str(
                &std::env::var("ETH_ADDRESS").expect("Must provide an address for this server!"),
            )
            .expect("Must provide a VIABLE address for this server");

            let websocket_key = std::env::var("WEB3_HOST").expect("Must provide a WEB3_HOST");

            Web3Config {
                account: my_address,
                websocket_key,
            }
        };

        Self {
            #[cfg(not(test))]
            sg_client: sendgrid::SGClient::new(my_secret_key),
            pool,
            #[cfg(feature = "web3")]
            web3_config,
        }
    }

    pub fn hash(s: &str) -> String {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish().to_string()
    }

    pub async fn prepare<T>(
        &self,
        email: &str,
        sec: &str,
        data: &T,
        #[cfg(feature = "web3")] contract_address: &str,
    ) -> HttpResponse
    where
        T: serde::Serialize,
    {
        #[cfg(feature = "web3")]
        let contract_address = contract_address.to_string();
        #[cfg(not(feature = "web3"))]
        let contract_address = "".to_string();

        let res = sqlx::query!(
            "INSERT INTO prepare (email, secret_component, data, contract_address) VALUES ($1, $2, $3, $4) RETURNING id",
            Self::hash(email),
            &sec,
            serde_json::to_value(data).expect("Could not serialize data"),
            &contract_address,
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

        if cfg!(feature = "development") {
            true
        } else {
            totp.check(otp, time)
        }
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
        contract_address: &str,
    ) -> Option<HttpResponse> {
        sqlx::query!("INSERT INTO authenticated (email, secret_component, status, data, contract_address) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (email) DO UPDATE SET secret_component = EXCLUDED.secret_component, data = EXCLUDED.data;",
                        Self::hash(email),
                        secret_component,
                        VerificationStatus::Verified as VerificationStatus,
                        data,
                        contract_address
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

    pub async fn get_prepared(
        &self,
        email: &str,
    ) -> Vec<(String, String, serde_json::Value, String)> {
        sqlx::query!(
            "SELECT id, secret_component, data, contract_address from prepare WHERE email=$1",
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
                rec.contract_address.clone() as Option<String>,
            )
        })
        .filter(|(a, b, c, d)| a.and(b.as_ref()).and(c.as_ref()).and(d.as_ref()).is_some())
        .map(|(id, sec, data, addr)| {
            (
                id.unwrap().to_string(),
                sec.unwrap(),
                data.unwrap(),
                addr.unwrap(),
            )
        })
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

    #[cfg(feature = "web3")]
    fn _local_extract_abi(contract_address: &str) -> Result<String, std::io::Error> {
        let versions =
            std::fs::read_to_string("/Users/benjcape/curr-term/cs98/etherscan/cpass/versions")
                .expect("Could not find Version file");

        let version = versions
            .split('\n')
            .map(|row| row.split('|'))
            .find_map(|mut row| (row.next() == Some(contract_address)).then(|| row.next()))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not find Version".to_string(),
                )
            })?
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not find Version".to_string(),
                )
            })?;

        std::fs::read_to_string(format!(
            "/Users/benjcape/curr-term/cs98/etherscan/cpass/{}/abi.json",
            version
        ))
    }

    pub(crate) async fn web2_handler(secret_component: Option<String>) -> Option<HttpResponse> {
        tracing::info!(ty = "web2", "Successful authentication");
        Some(HttpResponseBuilder::new(StatusCode::OK).json(secret_component))
    }

    #[cfg(feature = "web3")]
    pub(crate) async fn web3_handler(
        web3_config: &Web3Config,
        secret_component: Option<String>,
        contract_address: &str,
        destination_address: &str,
    ) -> Option<HttpResponse> {
        let Web3Config {
            account,
            websocket_key,
        } = web3_config;

        let web3 = match Self::_build_web3_client(websocket_key).await {
            Ok(web3) => web3,
            Err(e) => return Some(e),
        };

        let abi = match Self::_local_extract_abi(contract_address) {
            Ok(abi) => abi,
            Err(e) => {
                return Some(
                    HttpResponseBuilder::new(StatusCode::INTERNAL_SERVER_ERROR).json(e.to_string()),
                )
            }
        };

        let contract = web3::contract::Contract::from_json(
            web3.eth(),
            web3::types::Address::from_str(contract_address).expect("Invalid contract address"),
            abi.as_bytes(),
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
                "commit",
                (
                    web3::types::Bytes(
                        secret_component
                            .clone()
                            .unwrap_or_default()
                            .as_bytes()
                            .to_vec(),
                    ),
                    web3::types::Address::from_str(destination_address).unwrap(),
                ),
                *account,
                web3::contract::Options {
                    gas: Some(6721975.into()),
                    ..Default::default()
                },
            )
            .await;

        match res {
            Ok(_) => None,
            Err(e) => Some(HttpResponseBuilder::new(StatusCode::BAD_REQUEST).json(e.to_string())),
        }
    }
}
