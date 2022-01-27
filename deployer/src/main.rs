use hex_literal::hex;

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

    let bytecode = include_str!("../../build/Counter.bin");

    let contract =
        web3::contract::Contract::deploy(web3.eth(), include_bytes!("../../build/Counter.abi"))?
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
        include_bytes!("../../build/Counter.abi"),
    )?;

    Ok(())
}
