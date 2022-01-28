use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;

async fn hello_world(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("Hello, World".into()))
}

async fn start_server(i: u16) {
    // We'll bind to 127.0.0.1:3000
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000 + i));

    // A `Service` is needed for every connection, so this
    // creates one from our `hello_world` function.
    let make_svc = make_service_fn(|_conn| async {
        // service_fn converts our function into a `Service`
        Ok::<_, Infallible>(service_fn(hello_world))
    });

    let server = Server::bind(&addr).serve(make_svc);

    server.await.unwrap();
}

#[tokio::main]
async fn main() -> web3::contract::Result<()> {
    // Make the "Counter" an argument
    let _ = env_logger::try_init();
    dotenv::dotenv().map_err(|e| web3::Error::Decoder(e.to_string()))?;

    let url = dotenv::var("WEB3_URL").unwrap_or_else(|_| "http://localhost:7545".into());

    let transport = web3::transports::Http::new(&url)?;
    let web3 = web3::Web3::new(transport);

    let accounts = web3.eth().accounts().await?;

    let owner = accounts[0];

    let bytecode = include_str!("../build/Counter.bin");

    let contract =
        web3::contract::Contract::deploy(web3.eth(), include_bytes!("../build/Counter.abi"))?
            .confirmations(0)
            .options(web3::contract::Options::with(|opt| {
                opt.value = Some(0.into());
                opt.gas_price = Some(5.into());
                opt.gas = Some(3_000_000.into());
            }))
            .execute(bytecode, (), owner)
            .await?;

    let contract = web3::contract::Contract::from_json(
        web3.eth(),
        contract.address(),
        include_bytes!("../build/Counter.abi"),
    )?;

    let mut i = 0;

    let mut servers = vec![];

    while i < 5 {
        servers.push(start_server(i));

        i += 1;
    }

    futures::future::join_all(servers).await;

    Ok(())
}
