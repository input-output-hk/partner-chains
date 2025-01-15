//! Integration tests for partner-chains smart contracts.
//! Public methods are tested with use of the cardano-node-ogmios test image,
//! that provides a fast single node Cardano chain.
//!
//! Dockerfile for the test image is present in the 'docker' directory.
//! In case of change to the supported cardano-node or ogmios,
//! it should be updated accordingly and pushed to the registry.

use cardano_serialization_lib::NetworkIdKind;
use hex_literal::hex;
use ogmios_client::{
	jsonrpsee::{client_for_url, OgmiosClients},
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use partner_chains_cardano_offchain::{
	await_tx::{AwaitTx, FixedDelayRetries},
	d_param, init_governance, permissioned_candidates,
	register::Register,
	reserve, scripts_data, update_governance,
};
use sidechain_domain::{
	AdaBasedStaking, AssetName, AuraPublicKey, CandidateRegistration, DParameter, GrandpaPublicKey,
	MainchainAddressHash, MainchainPrivateKey, MainchainPublicKey, MainchainSignature, McTxHash,
	PermissionedCandidateData, PolicyId, SidechainPublicKey, SidechainSignature, TokenId, UtxoId,
};
use std::time::Duration;
use testcontainers::{clients::Cli, Container, GenericImage};
use tokio_retry::{strategy::FixedInterval, Retry};

const TEST_IMAGE: &str = "ghcr.io/input-output-hk/smart-contracts-tests-cardano-node-ogmios";

const TEST_IMAGE_TAG: &str = "v10.1.4-v6.11.0";

const GOVERNANCE_AUTHORITY: MainchainAddressHash =
	MainchainAddressHash(hex!("e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"));

const GOVERNANCE_AUTHORITY_PAYMENT_KEY: MainchainPrivateKey =
	MainchainPrivateKey(hex!("d0a6c5c921266d15dc8d1ce1e51a01e929a686ed3ec1a9be1145727c224bf386"));

const GOVERNANCE_AUTHORITY_ADDRESS: &str =
	"addr_test1vr5vxqpnpl3325cu4zw55tnapjqzzx78pdrnk8k5j7wl72c6y08nd";

const EVE_PAYMENT_KEY: MainchainPrivateKey =
	MainchainPrivateKey(hex!("34a6ce19688e950b58ea73803a00db61d0505ba10d65756d85f27c37d24c06af"));

const EVE_PUBLIC_KEY: MainchainPublicKey =
	MainchainPublicKey(hex!("a5ab6e82531cac3480cf7ff360f38a0beeea93cabfdd1ed0495e0423f7875c57"));

const EVE_PUBLIC_KEY_HASH: MainchainAddressHash =
	MainchainAddressHash(hex!("84ba05c28879b299a8377e62128adc7a0e0df3ac438ff95efc7c8443"));

const EVE_ADDRESS: &str = "addr_test1vzzt5pwz3pum9xdgxalxyy52m3aqur0n43pcl727l37ggscl8h7v8";

const REWARDS_TOKEN_POLICY_ID: PolicyId =
	PolicyId(hex!("1fab25f376bc49a181d03a869ee8eaa3157a3a3d242a619ca7995b2b"));

// Reward token
const REWARDS_TOKEN_ASSET_NAME_STR: &str = "52657761726420746f6b656e";

const INITIAL_DEPOSIT_AMOUNT: u64 = 500000;
const DEPOSIT_AMOUNT: u64 = 100000;

#[tokio::test]
async fn governance_flow() {
	let image = GenericImage::new(TEST_IMAGE, TEST_IMAGE_TAG);
	let cli = Cli::default();
	let container = cli.run(image);
	let client = initialize(&container).await;
	let genesis_utxo = run_init_goveranance(&client).await;
	let _ = run_update_goveranance(&client, genesis_utxo).await;
	assert!(run_upsert_d_param(genesis_utxo, 0, 1, EVE_PAYMENT_KEY, &client).await.is_some());
}

#[tokio::test]
async fn upsert_d_param() {
	let image = GenericImage::new(TEST_IMAGE, TEST_IMAGE_TAG);
	let client = Cli::default();
	let container = client.run(image);
	let client = initialize(&container).await;
	let genesis_utxo = run_init_goveranance(&client).await;
	assert!(run_upsert_d_param(genesis_utxo, 0, 1, GOVERNANCE_AUTHORITY_PAYMENT_KEY, &client)
		.await
		.is_some());
	assert!(run_upsert_d_param(genesis_utxo, 0, 1, GOVERNANCE_AUTHORITY_PAYMENT_KEY, &client)
		.await
		.is_none());
	assert!(run_upsert_d_param(genesis_utxo, 1, 1, GOVERNANCE_AUTHORITY_PAYMENT_KEY, &client)
		.await
		.is_some())
}

#[tokio::test]
async fn upsert_permissioned_candidates() {
	let image = GenericImage::new(TEST_IMAGE, TEST_IMAGE_TAG);
	let client = Cli::default();
	let container = client.run(image);
	let client = initialize(&container).await;
	let genesis_utxo = run_init_goveranance(&client).await;
	assert!(run_upsert_permissioned_candidates(genesis_utxo, 77, &client).await.is_some());
	assert!(run_upsert_permissioned_candidates(genesis_utxo, 77, &client).await.is_none());
	assert!(run_upsert_permissioned_candidates(genesis_utxo, 231, &client).await.is_some())
}

#[tokio::test]
async fn reserve_management_scenario() {
	let image = GenericImage::new(TEST_IMAGE, TEST_IMAGE_TAG);
	let client = Cli::default();
	let container = client.run(image);
	let client = initialize(&container).await;
	let genesis_utxo = run_init_goveranance(&client).await;
	let txs = run_init_reserve_management(genesis_utxo, &client).await;
	assert_eq!(txs.len(), 3);
	let txs = run_init_reserve_management(genesis_utxo, &client).await;
	assert_eq!(txs.len(), 0);
	let _ = run_create_reserve_management(genesis_utxo, &client).await;
	assert_reserve_deposited(genesis_utxo, INITIAL_DEPOSIT_AMOUNT, &client).await;
	run_deposit_to_reserve(genesis_utxo, &client).await;
	assert_reserve_deposited(genesis_utxo, INITIAL_DEPOSIT_AMOUNT + DEPOSIT_AMOUNT, &client).await;
}

#[tokio::test]
async fn register() {
	let image = GenericImage::new(TEST_IMAGE, TEST_IMAGE_TAG);
	let client = Cli::default();
	let container = client.run(image);
	let client = initialize(&container).await;
	let genesis_utxo = run_init_goveranance(&client).await;
	let signature = SidechainSignature([21u8; 33].to_vec());
	let other_signature = SidechainSignature([121u8; 33].to_vec());
	assert!(run_register(genesis_utxo, signature.clone(), &client).await.is_some());
	assert!(run_register(genesis_utxo, signature, &client).await.is_none());
	assert!(run_register(genesis_utxo, other_signature, &client).await.is_some());
}

async fn initialize<'a>(container: &Container<'a, GenericImage>) -> OgmiosClients {
	let ogmios_port = container.get_host_port_ipv4(1337);
	println!("Ogmios port: {}", ogmios_port);

	let client = await_ogmios(ogmios_port).await.unwrap();
	println!("Ogmios is up");
	let _ = initial_transaction(&client).await.unwrap();
	println!("Initial transaction confirmed");
	client
}

async fn await_ogmios(ogmios_port: u16) -> Result<OgmiosClients, String> {
	let url = format!("ws://localhost:{}", ogmios_port);
	Retry::spawn(FixedInterval::new(Duration::from_millis(100)).take(1000), || async {
		let client = client_for_url(&url).await?;
		let _ = client.shelley_genesis_configuration().await.map_err(|e| e.to_string())?;
		Ok(client)
	})
	.await
}

/// initial transaction was obtained with cardano-cli and it sends funds to:
/// * governance authority: addr_test1vr5vxqpnpl3325cu4zw55tnapjqzzx78pdrnk8k5j7wl72c6y08nd (2 x UTXO)
/// * governance authority: 1000000 REWARDS_TOKEN
/// * "dave" address: addr_test1vphpcf32drhhznv6rqmrmgpuwq06kug0lkg22ux777rtlqst2er0r
/// * "eve" address: addr_test1vzzt5pwz3pum9xdgxalxyy52m3aqur0n43pcl727l37ggscl8h7v8
///
/// Its hash is 0x61ca664e056ce49a9d4fd2fb3aa2b750ea753fe4ad5c9e6167482fd88394cf7d
async fn initial_transaction<T: Transactions + QueryUtxoByUtxoId>(
	client: &T,
) -> Result<McTxHash, ()> {
	let signed_tx_bytes  = hex!("84a400d9010281825820781cb948a37c7c38b43872af9b1e22135a94826eafd3740260a6db0a303885d800018682581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b1a3b9aca0082581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b1a3b9aca0082581d606e1c262a68ef714d9a18363da03c701fab710ffd90a570def786bf821a3b9aca0082581d6084ba05c28879b299a8377e62128adc7a0e0df3ac438ff95efc7c84431a3b9aca0082581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b1b006a8e81dfdc1f4082581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b821a00989680a1581c1fab25f376bc49a181d03a869ee8eaa3157a3a3d242a619ca7995b2ba14c52657761726420746f6b656e1a000f4240021a000f424009a1581c1fab25f376bc49a181d03a869ee8eaa3157a3a3d242a619ca7995b2ba14c52657761726420746f6b656e1a000f4240a200d9010282825820e6ceac21f27c463f9065fafdc62883d7e52f6a376b498b8838ba513e44c74eca58406c09c0a1bf773bbcb91cdaff46a6d7548268d2f1dbc7c203dbf4e1f1cd031895faede520f10d7758b8279d4c68484f1a055792e0881a5becf91bf5d8e861410b825820fc014cb5f071f5d6a36cb5a7e5f168c86555989445a23d4abec33d280f71aca4584083c00332dc76cd42ed33610f8a56efa0ced659b3752e5f80ee8176e726c48715c2cbdb544bf4eb4d424902d2861ab1c7deabfcfe795f779795ed9abc3dcfa10f01d90102818200581ce8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2bf5f6");
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

async fn run_update_goveranance<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
>(
	client: &T,
	genesis_utxo: UtxoId,
) {
	let _ = update_governance::run_update_governance(
		EVE_PUBLIC_KEY_HASH,
		GOVERNANCE_AUTHORITY_PAYMENT_KEY,
		genesis_utxo,
		client,
		FixedDelayRetries::new(Duration::from_millis(500), 100),
	)
	.await
	.unwrap();
}

async fn run_upsert_d_param<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
>(
	genesis_utxo: UtxoId,
	num_permissioned_candidates: u16,
	num_registered_candidates: u16,
	pkey: MainchainPrivateKey,
	client: &T,
) -> Option<McTxHash> {
	let tx_hash = d_param::upsert_d_param(
		genesis_utxo,
		&DParameter { num_permissioned_candidates, num_registered_candidates },
		pkey.0,
		client,
		&FixedDelayRetries::new(Duration::from_millis(500), 100),
	)
	.await
	.unwrap();
	if let Some(tx_hash) = tx_hash {
		FixedDelayRetries::new(Duration::from_millis(500), 100)
			.await_tx_output(client, UtxoId::new(tx_hash.0, 0))
			.await
			.unwrap()
	};
	tx_hash
}

async fn run_upsert_permissioned_candidates<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
>(
	genesis_utxo: UtxoId,
	candidate: u8,
	client: &T,
) -> Option<McTxHash> {
	let candidates = vec![PermissionedCandidateData {
		sidechain_public_key: SidechainPublicKey([candidate; 33].to_vec()),
		aura_public_key: AuraPublicKey([candidate; 32].to_vec()),
		grandpa_public_key: GrandpaPublicKey([candidate; 32].to_vec()),
	}];
	let tx_hash = permissioned_candidates::upsert_permissioned_candidates(
		genesis_utxo,
		&candidates,
		GOVERNANCE_AUTHORITY_PAYMENT_KEY.0,
		client,
		&FixedDelayRetries::new(Duration::from_millis(500), 100),
	)
	.await
	.unwrap();
	if let Some(tx_hash) = tx_hash {
		FixedDelayRetries::new(Duration::from_millis(500), 100)
			.await_tx_output(client, UtxoId::new(tx_hash.0, 0))
			.await
			.unwrap()
	};
	tx_hash
}

async fn run_init_reserve_management<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
>(
	genesis_utxo: UtxoId,
	client: &T,
) -> Vec<McTxHash> {
	reserve::init::init_reserve_management(
		genesis_utxo,
		GOVERNANCE_AUTHORITY_PAYMENT_KEY.0,
		client,
		&FixedDelayRetries::new(Duration::from_millis(500), 100),
	)
	.await
	.unwrap()
}

async fn run_create_reserve_management<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
>(
	genesis_utxo: UtxoId,
	client: &T,
) -> McTxHash {
	reserve::create::create_reserve_utxo(
		reserve::create::ReserveParameters {
			initial_incentive: 100,
			total_accrued_function_script_hash: PolicyId([233u8; 28]),
			token: TokenId::AssetId {
				policy_id: REWARDS_TOKEN_POLICY_ID,
				asset_name: AssetName::from_hex_unsafe(REWARDS_TOKEN_ASSET_NAME_STR),
			},
			initial_deposit: INITIAL_DEPOSIT_AMOUNT,
		},
		genesis_utxo,
		GOVERNANCE_AUTHORITY_PAYMENT_KEY.0,
		client,
		&FixedDelayRetries::new(Duration::from_millis(500), 100),
	)
	.await
	.unwrap()
}

async fn run_deposit_to_reserve<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
>(
	genesis_utxo: UtxoId,
	client: &T,
) -> () {
	reserve::deposit::deposit_to_reserve(
		reserve::deposit::TokenAmount {
			token: TokenId::AssetId {
				policy_id: REWARDS_TOKEN_POLICY_ID,
				asset_name: AssetName::from_hex_unsafe(REWARDS_TOKEN_ASSET_NAME_STR),
			},
			amount: 100000,
		},
		genesis_utxo,
		GOVERNANCE_AUTHORITY_PAYMENT_KEY.0,
		client,
		&FixedDelayRetries::new(Duration::from_millis(500), 100),
	)
	.await
	.unwrap();
}

async fn run_register<T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId>(
	genesis_utxo: UtxoId,
	partnerchain_signature: SidechainSignature,
	client: &T,
) -> Option<McTxHash> {
	let eve_utxos = client.query_utxos(&[EVE_ADDRESS.to_string()]).await.unwrap();
	let registration_utxo = eve_utxos.first().unwrap().utxo_id();
	client
		.register(
			genesis_utxo,
			&CandidateRegistration {
				stake_ownership: AdaBasedStaking {
					pub_key: EVE_PUBLIC_KEY,
					signature: MainchainSignature(vec![19u8; 32]),
				},
				partner_chain_pub_key: SidechainPublicKey([20u8; 32].to_vec()),
				partner_chain_signature: partnerchain_signature,
				own_pkh: EVE_PUBLIC_KEY_HASH,
				registration_utxo,
				aura_pub_key: AuraPublicKey([22u8; 32].to_vec()),
				grandpa_pub_key: GrandpaPublicKey([23u8; 32].to_vec()),
			},
			EVE_PAYMENT_KEY,
		)
		.await
		.unwrap()
}

async fn assert_token_amount_eq<T: QueryLedgerState>(
	address: &str,
	token_policy_id: &PolicyId,
	token_asset_name: &AssetName,
	expected_amount: u64,
	client: &T,
) {
	let utxos = client.query_utxos(&[address.to_string()]).await.unwrap();
	assert!(
		utxos.into_iter().any(|utxo| {
			utxo.value
				.native_tokens
				.get(&token_policy_id.0)
				.and_then(|assets| assets.iter().find(|a| a.name == token_asset_name.0.to_vec()))
				.is_some_and(|asset| asset.amount == expected_amount.into())
		}),
		"Expected to find UTXO with {} of {}.{} at {}",
		expected_amount,
		hex::encode(&token_policy_id.0),
		hex::encode(&token_asset_name.0),
		address,
	);
}

async fn assert_reserve_deposited<T: QueryLedgerState>(
	genesis_utxo: UtxoId,
	amount: u64,
	client: &T,
) {
	let data = scripts_data::get_scripts_data(genesis_utxo, NetworkIdKind::Testnet).unwrap();
	assert_token_amount_eq(
		&data.addresses.reserve_validator,
		&REWARDS_TOKEN_POLICY_ID,
		&AssetName::from_hex_unsafe(REWARDS_TOKEN_ASSET_NAME_STR),
		amount,
		client,
	)
	.await;
}
