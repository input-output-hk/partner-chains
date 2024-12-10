use hex_literal::hex;
use jsonrpsee::http_client::HttpClient;
use ogmios_client::query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId};
use ogmios_client::transactions::Transactions;
use ogmios_client::{query_network::QueryNetwork, OgmiosClientError};
use partner_chains_cardano_offchain::await_tx::{AwaitTx, FixedDelayRetries};
use partner_chains_cardano_offchain::{d_param, init_governance};
use sidechain_domain::{DParameter, MainchainAddressHash, MainchainPrivateKey, McTxHash, UtxoId};
use std::time::Duration;
use testcontainers::Container;
use testcontainers::{clients::Cli, GenericImage};
use tokio_retry::{strategy::FixedInterval, Retry};

const TEST_IMAGE: &str = "partner-chains-smart-contracts-tests-cardano-node-ogmios";

const TEST_IMAGE_TAG: &str = "v10.2.1-v6.9.0";

const GOVERNANCE_AUTHORITY: MainchainAddressHash =
	MainchainAddressHash(hex!("e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"));

const GOVERNANCE_AUTHORITY_PAYMENT_KEY: MainchainPrivateKey =
	MainchainPrivateKey(hex!("d0a6c5c921266d15dc8d1ce1e51a01e929a686ed3ec1a9be1145727c224bf386"));

const GOVERNANCE_AUTHORITY_ADDRESS: &str =
	"addr_test1vr5vxqpnpl3325cu4zw55tnapjqzzx78pdrnk8k5j7wl72c6y08nd";

#[tokio::test]
async fn init_goveranance() {
	let image = GenericImage::new(TEST_IMAGE, TEST_IMAGE_TAG);
	let client = Cli::default();
	let container = client.run(image);
	let client = initialize(&container).await;
	let _ = run_init_goveranance(&client).await;
	()
}

#[tokio::test]
async fn upsert_d_param() {
	let image = GenericImage::new(TEST_IMAGE, TEST_IMAGE_TAG);
	let client = Cli::default();
	let container = client.run(image);
	let client = initialize(&container).await;
	let genesis_utxo = run_init_goveranance(&client).await;
	assert!(run_upsert_d_param(genesis_utxo, 0, 1, &client).await.is_some());
	assert!(run_upsert_d_param(genesis_utxo, 0, 1, &client).await.is_none());
	assert!(run_upsert_d_param(genesis_utxo, 1, 1, &client).await.is_some())
}

async fn initialize<'a>(container: &Container<'a, GenericImage>) -> HttpClient {
	let ogmios_port = container.get_host_port_ipv4(1337);
	println!("ogmios port: {}", ogmios_port);
	let client = HttpClient::builder()
		.build(format!("http://localhost:{}", ogmios_port))
		.unwrap();

	await_ogmios(&client).await.unwrap();
	let _ = initial_transaction(&client).await.unwrap();
	client
}

async fn await_ogmios<T: QueryNetwork>(client: &T) -> Result<(), OgmiosClientError> {
	Retry::spawn(FixedInterval::new(Duration::from_millis(100)).take(1000), || async {
		client.shelley_genesis_configuration().await.map(|_| ())
	})
	.await
}

/// initial transaction was obtained with cardano-cli and it sends funds to:
/// * goveranance authority: addr_test1vr5vxqpnpl3325cu4zw55tnapjqzzx78pdrnk8k5j7wl72c6y08nd (2 x UTXO)
/// * "dave" address: addr_test1vphpcf32drhhznv6rqmrmgpuwq06kug0lkg22ux777rtlqst2er0r
/// * "eve" address: addr_test1vzzt5pwz3pum9xdgxalxyy52m3aqur0n43pcl727l37ggscl8h7v8
/// Its hash is 0xc389187c6cabf1cd2ca64cf8c76bf57288eb9c02ced6781935b810a1d0e7fbb4
async fn initial_transaction<T: Transactions + QueryUtxoByUtxoId>(
	client: &T,
) -> Result<McTxHash, ()> {
	let signed_tx_bytes  = hex!("84a300d9010281825820781cb948a37c7c38b43872af9b1e22135a94826eafd3740260a6db0a303885d800018582581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b1a3b9aca0082581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b1a3b9aca0082581d606e1c262a68ef714d9a18363da03c701fab710ffd90a570def786bf821a3b9aca0082581d6084ba05c28879b299a8377e62128adc7a0e0df3ac438ff95efc7c84431a3b9aca0082581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b1b006a8e81e074b5c0021a000f4240a100d9010281825820e6ceac21f27c463f9065fafdc62883d7e52f6a376b498b8838ba513e44c74eca58406d60019f2589001024a15c300e034de74998a5b7bc995a8d0f21c2fdfc0cd7c9106d77e6507d5b708434d0616a7b1a53ec0341dffc553e2ab8c9be15197d0503f5f6");
	let tx_hash = client
		.submit_transaction(&signed_tx_bytes)
		.await
		.map_err(|_| ())
		.map(|response| McTxHash(response.transaction.id))?;
	FixedDelayRetries::new(Duration::from_millis(500), 100)
		.await_tx_output(client, UtxoId::new(tx_hash.0, 0))
		.await
		.map_err(|_| ())?;
	Ok(tx_hash)
}

async fn run_init_goveranance<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
>(
	client: &T,
) -> UtxoId {
	let governance_utxos =
		client.query_utxos(&[GOVERNANCE_AUTHORITY_ADDRESS.to_string()]).await.unwrap();
	let genesis_utxo = governance_utxos.first().cloned().unwrap().utxo_id();
	let _ = init_governance::run_init_governance(
		GOVERNANCE_AUTHORITY,
		GOVERNANCE_AUTHORITY_PAYMENT_KEY,
		Some(genesis_utxo),
		client,
		FixedDelayRetries::new(Duration::from_millis(500), 100),
	)
	.await
	.unwrap();
	genesis_utxo
}

async fn run_upsert_d_param<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
>(
	genesis_utxo: UtxoId,
	num_permissioned_candidates: u16,
	num_registered_candidates: u16,
	client: &T,
) -> Option<McTxHash> {
	let tx_hash = d_param::upsert_d_param(
		genesis_utxo,
		&DParameter { num_permissioned_candidates, num_registered_candidates },
		GOVERNANCE_AUTHORITY_PAYMENT_KEY.0,
		client,
	)
	.await
	.unwrap();
	match tx_hash {
		Some(tx_hash) => FixedDelayRetries::new(Duration::from_millis(500), 100)
			.await_tx_output(client, UtxoId::new(tx_hash.0, 0))
			.await
			.unwrap(),
		None => (),
	};
	tx_hash
}
