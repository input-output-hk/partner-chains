mod dereg;
mod ogmios;
mod plutus_data;
mod tx;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	dereg::deregister(pallas_addresses::Network::Testnet).await
}
