use actix_web::{get, post, web, HttpRequest, HttpResponseBuilder, Responder};
use hyper::StatusCode;
use serde::Serialize;
use web3::types::Address;

mod utils {
    use web3::{transports::Http, types::Address};

    pub async fn setup(address: &Address) -> web3::contract::Contract<Http> {
        let url = dotenv::var("WEB3_URL").unwrap_or_else(|_| "http://localhost:7545".into());

        let transport = web3::transports::Http::new(&url).unwrap();
        let web3 = web3::Web3::new(transport);

        web3::contract::Contract::from_json(
            web3.eth(),
            *address,
            include_bytes!("../../build/Counter.abi"),
        )
        .unwrap()
    }
}

#[derive(Serialize)]
struct CounterResponse {
    counter: i32,
}

#[get("/")]
pub async fn index(req: HttpRequest) -> impl Responder {
    let (address, account) = req.app_data::<(Address, Address)>().unwrap();

    let contract = utils::setup(address).await;

    let counter: i32 = contract
        .query("counter", (), *account, Default::default(), None)
        .await
        .unwrap();

    web::Json(CounterResponse { counter })
}

#[post("/inc")]
pub async fn inc(req: HttpRequest) -> impl Responder {
    let (address, account) = req.app_data::<(Address, Address)>().unwrap();

    let contract = utils::setup(address).await;

    contract
        .call("inc", (), *account, Default::default())
        .await
        .unwrap();

    HttpResponseBuilder::new(StatusCode::ACCEPTED)
}

#[post("/dec")]
pub async fn dec(req: HttpRequest) -> impl Responder {
    let (address, account) = req.app_data::<(Address, Address)>().unwrap();

    let contract = utils::setup(address).await;

    contract
        .call("dec", (), *account, Default::default())
        .await
        .unwrap();

    HttpResponseBuilder::new(StatusCode::ACCEPTED)
}
