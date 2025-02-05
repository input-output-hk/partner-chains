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
	cardano_keys::CardanoPaymentSigningKey,
	d_param, init_governance, permissioned_candidates,
	register::Register,
	reserve::{self, release::release_reserve_funds, TokenAmount},
	scripts_data, update_governance,
};
use partner_chains_plutus_data::reserve::ReserveDatum;
use sidechain_domain::{
	AdaBasedStaking, AssetId, AssetName, AuraPublicKey, CandidateRegistration, DParameter,
	GrandpaPublicKey, MainchainKeyHash, MainchainPublicKey, MainchainSignature, McTxHash,
	PermissionedCandidateData, PolicyId, SidechainPublicKey, SidechainSignature, UtxoId, UtxoIndex,
};
use std::time::Duration;
use testcontainers::{clients::Cli, Container, GenericImage};
use tokio_retry::{strategy::FixedInterval, Retry};

const TEST_IMAGE: &str = "ghcr.io/input-output-hk/smart-contracts-tests-cardano-node-ogmios";

const TEST_IMAGE_TAG: &str = "v10.1.4-v6.11.0";

const GOVERNANCE_AUTHORITY: MainchainKeyHash =
	MainchainKeyHash(hex!("e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"));

fn governance_authority_payment_key() -> CardanoPaymentSigningKey {
	CardanoPaymentSigningKey::from_normal_bytes(hex!(
		"d0a6c5c921266d15dc8d1ce1e51a01e929a686ed3ec1a9be1145727c224bf386"
	))
	.unwrap()
}

const GOVERNANCE_AUTHORITY_ADDRESS: &str =
	"addr_test1vr5vxqpnpl3325cu4zw55tnapjqzzx78pdrnk8k5j7wl72c6y08nd";

fn eve_payment_key() -> CardanoPaymentSigningKey {
	CardanoPaymentSigningKey::from_normal_bytes(hex!(
		"34a6ce19688e950b58ea73803a00db61d0505ba10d65756d85f27c37d24c06af"
	))
	.unwrap()
}

const EVE_PUBLIC_KEY: MainchainPublicKey =
	MainchainPublicKey(hex!("a5ab6e82531cac3480cf7ff360f38a0beeea93cabfdd1ed0495e0423f7875c57"));

const EVE_PUBLIC_KEY_HASH: MainchainKeyHash =
	MainchainKeyHash(hex!("84ba05c28879b299a8377e62128adc7a0e0df3ac438ff95efc7c8443"));

const EVE_ADDRESS: &str = "addr_test1vzzt5pwz3pum9xdgxalxyy52m3aqur0n43pcl727l37ggscl8h7v8";

const V_FUNCTION_HASH: PolicyId =
	PolicyId(hex!("ef1eb7b85327a8460799025a5affd0a8d8015731e9aacd5d1106a82b"));
const V_FUNCTION_UTXO: UtxoId = UtxoId {
	tx_hash: McTxHash(hex!("f8fbe7316561e57de9ecd1c86ee8f8b512a314ba86499ba9a584bfa8fe2edc8d")),
	index: UtxoIndex(6),
};

const REWARDS_TOKEN_POLICY_ID: PolicyId =
	PolicyId(hex!("1fab25f376bc49a181d03a869ee8eaa3157a3a3d242a619ca7995b2b"));

// Reward token
const REWARDS_TOKEN_ASSET_NAME_STR: &str = "52657761726420746f6b656e";

const INITIAL_DEPOSIT_AMOUNT: u64 = 500000;
const DEPOSIT_AMOUNT: u64 = 100000;
const RELEASE_AMOUNT: u64 = 90;

const UPDATED_TOTAL_ACCRUED_FUNCTION_SCRIPT_HASH: PolicyId = PolicyId([234u8; 28]);

#[tokio::test]
async fn governance_flow() {
	let image = GenericImage::new(TEST_IMAGE, TEST_IMAGE_TAG);
	let cli = Cli::default();
	let container = cli.run(image);
	let client = initialize(&container).await;
	let genesis_utxo = run_init_goveranance(&client).await;
	let _ = run_update_goveranance(&client, genesis_utxo).await;
	assert!(run_upsert_d_param(genesis_utxo, 0, 1, &eve_payment_key(), &client)
		.await
		.is_some());
}

#[tokio::test]
async fn upsert_d_param() {
	let image = GenericImage::new(TEST_IMAGE, TEST_IMAGE_TAG);
	let client = Cli::default();
	let container = client.run(image);
	let client = initialize(&container).await;
	let genesis_utxo = run_init_goveranance(&client).await;
	assert!(run_upsert_d_param(genesis_utxo, 0, 1, &governance_authority_payment_key(), &client)
		.await
		.is_some());
	assert!(run_upsert_d_param(genesis_utxo, 0, 1, &governance_authority_payment_key(), &client)
		.await
		.is_none());
	assert!(run_upsert_d_param(genesis_utxo, 1, 1, &governance_authority_payment_key(), &client)
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
	let _ = run_create_reserve_management(genesis_utxo, V_FUNCTION_HASH, &client).await;
	assert_reserve_deposited(genesis_utxo, INITIAL_DEPOSIT_AMOUNT, &client).await;
	run_deposit_to_reserve(genesis_utxo, &client).await;
	assert_reserve_deposited(genesis_utxo, INITIAL_DEPOSIT_AMOUNT + DEPOSIT_AMOUNT, &client).await;
	run_release_reserve_funds(genesis_utxo, RELEASE_AMOUNT, V_FUNCTION_UTXO, &client).await;
	assert_reserve_deposited(
		genesis_utxo,
		INITIAL_DEPOSIT_AMOUNT + DEPOSIT_AMOUNT - RELEASE_AMOUNT,
		&client,
	)
	.await;
	assert_illiquid_supply(genesis_utxo, RELEASE_AMOUNT, &client).await;
	run_update_reserve_settings_management(
		genesis_utxo,
		UPDATED_TOTAL_ACCRUED_FUNCTION_SCRIPT_HASH,
		&client,
	)
	.await;
	assert_mutable_settings_eq(genesis_utxo, UPDATED_TOTAL_ACCRUED_FUNCTION_SCRIPT_HASH, &client)
		.await;
	run_handover_reserve(genesis_utxo, &client).await.unwrap();
	assert_reserve_handed_over(genesis_utxo, INITIAL_DEPOSIT_AMOUNT + DEPOSIT_AMOUNT, &client)
		.await;
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
/// * addr_test1vzuasm5nqzh7n909f7wang7apjprpg29l2f9sk6shlt84rqep6nyc - has attached V-function script
///
/// Its hash is 0xf8fbe7316561e57de9ecd1c86ee8f8b512a314ba86499ba9a584bfa8fe2edc8d
async fn initial_transaction<T: Transactions + QueryUtxoByUtxoId>(
	client: &T,
) -> Result<McTxHash, String> {
	let signed_tx_bytes = hex!("84a400d9010281825820781cb948a37c7c38b43872af9b1e22135a94826eafd3740260a6db0a303885d800018782581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b1a3b9aca0082581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b1a3b9aca0082581d606e1c262a68ef714d9a18363da03c701fab710ffd90a570def786bf821a3b9aca0082581d6084ba05c28879b299a8377e62128adc7a0e0df3ac438ff95efc7c84431a3b9aca0082581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b1b006a8e81df4388c082581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b821a00989680a1581c1fab25f376bc49a181d03a869ee8eaa3157a3a3d242a619ca7995b2ba14c52657761726420746f6b656e1a000f4240a300581d60b9d86e9300afe995e54f9dd9a3dd0c8230a145fa92585b50bfd67a8c011a0098968003d81859072b820259072659072301000033233223222253232335332232353232325333573466e1d20000021323232323232332212330010030023232325333573466e1d2000002132323232323232323232332323233323333323332332332222222222221233333333333300100d00c00b00a00900800700600500400300230013574202460026ae84044c00c8c8c8c94ccd5cd19b87480000084cc8848cc00400c008c070d5d080098029aba135744002260489201035054310035573c0046aae74004dd5000998018009aba100f23232325333573466e1d20000021323232333322221233330010050040030023232325333573466e1d20000021332212330010030023020357420026600803e6ae84d5d100089814a481035054310035573c0046aae74004dd51aba1004300835742006646464a666ae68cdc3a4000004224440062a666ae68cdc3a4004004264244460020086eb8d5d08008a999ab9a3370e9002001099091118010021aba100113029491035054310035573c0046aae74004dd51aba10023300175c6ae84d5d1001111919192999ab9a3370e900100108910008a999ab9a3370e9000001099091180100198029aba10011302a491035054310035573c0046aae74004dd50009aba20013574400226046921035054310035573c0046aae74004dd500098009aba100d30013574201860046004eb4cc00404cd5d080519980200a3ad35742012646464a666ae68cdc3a40000042646466442466002006004646464a666ae68cdc3a40000042664424660020060046600aeb4d5d080098021aba1357440022604c921035054310035573c0046aae74004dd51aba10033232325333573466e1d20000021332212330010030023300575a6ae84004c010d5d09aba2001130264901035054310035573c0046aae74004dd51aba1357440064646464a666ae68cdc3a400000420482a666ae68cdc3a4004004204a2604c921035054310035573c0046aae74004dd5000911919192999ab9a3370e9000001089110010a999ab9a3370e90010010990911180180218029aba100115333573466e1d20040021122200113026491035054310035573c0046aae74004dd500089810a49035054310035573c0046aae74004dd51aba10083300175c6ae8401c8c88c008dd60009813111999aab9f0012028233502730043574200460066ae88008084ccc00c044008d5d0802998008011aba1004300275c40024464460046eac004c09088cccd55cf800901311919a8131991091980080180118031aab9d001300535573c00260086ae8800cd5d080100f98099aba1357440026ae88004d5d10009aba2001357440026ae88004d5d10009aba2001357440026ae88004d5d100089808249035054310035573c0046aae74004dd51aba10073001357426ae8801c8c8c8c94ccd5cd19b87480000084c848888c00c014dd71aba100115333573466e1d20020021321222230010053008357420022a666ae68cdc3a400800426424444600400a600c6ae8400454ccd5cd19b87480180084c848888c010014c014d5d080089808249035054310035573c0046aae74004dd500091919192999ab9a3370e900000109909111111180280418029aba100115333573466e1d20020021321222222230070083005357420022a666ae68cdc3a400800426644244444446600c012010600a6ae84004dd71aba1357440022a666ae68cdc3a400c0042664424444444660040120106eb8d5d08009bae357426ae8800454ccd5cd19b87480200084cc8848888888cc004024020dd71aba1001375a6ae84d5d10008a999ab9a3370e90050010891111110020a999ab9a3370e900600108911111100189807a49035054310035573c0046aae74004dd500091919192999ab9a3370e9000001099091180100198029aba100115333573466e1d2002002132333222122333001005004003375a6ae84008dd69aba1001375a6ae84d5d10009aba20011300e4901035054310035573c0046aae74004dd500091919192999ab9a3370e900000109909118010019bae357420022a666ae68cdc3a400400426424460020066eb8d5d080089806a481035054310035573c0046aae74004dd500091919192999ab9a3370e900000109991091980080180118029aba1001375a6ae84d5d1000898062481035054310035573c0046aae74004dd500091919192999ab9a3370e900000109bae3574200226016921035054310035573c0046aae74004dd500089803a49035054310035573c0046aae74004dd5003111999a8009002919199ab9a337126602044a66a002290001109a801112999ab9a3371e004010260260022600c006600244444444444401066e0ccdc09a9a980091111111111100291001112999a80110a99a80108008b0b0b002a4181520e00e00ca006400a400a6eb401c48800848800440084c00524010350543500232633573800200424002600644a66a002290001109a8011119b800013006003122002122122330010040032323001001223300330020020014c01051a677485800001021a000f424009a1581c1fab25f376bc49a181d03a869ee8eaa3157a3a3d242a619ca7995b2ba14c52657761726420746f6b656e1a000f4240a200d9010282825820e6ceac21f27c463f9065fafdc62883d7e52f6a376b498b8838ba513e44c74eca5840ec9de986448bf5d618e060974a1864eb352387201f661ff2f2dc4b2a2b455de1987fa8a1b083c2a2760964524813bda68a59a28dd76ea7af01d50cdcba36be00825820fc014cb5f071f5d6a36cb5a7e5f168c86555989445a23d4abec33d280f71aca458409dc0ccc1dfac12fb1c82e72568d5f0a6384633842cb67c5ec0daafe3fe599902bfa2d5a1f72230dbecd104ecc1a8bcc5a981fb658448d1a7f8aead54678dd90401d90102818200581ce8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2bf5f6");
	let tx_hash = client
		.submit_transaction(&signed_tx_bytes)
		.await
		.map_err(|e| e.to_string())
		.map(|response| McTxHash(response.transaction.id))?;
	FixedDelayRetries::new(Duration::from_millis(500), 100)
		.await_tx_output(client, UtxoId::new(tx_hash.0, 0))
		.await
		.map_err(|e| e.to_string())?;
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
		&governance_authority_payment_key(),
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
		&governance_authority_payment_key(),
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
	pkey: &CardanoPaymentSigningKey,
	client: &T,
) -> Option<McTxHash> {
	let tx_hash = d_param::upsert_d_param(
		genesis_utxo,
		&DParameter { num_permissioned_candidates, num_registered_candidates },
		pkey,
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
		&governance_authority_payment_key(),
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
		&governance_authority_payment_key(),
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
	v_function_hash: PolicyId,
	client: &T,
) -> McTxHash {
	reserve::create::create_reserve_utxo(
		reserve::create::ReserveParameters {
			total_accrued_function_script_hash: v_function_hash,
			token: AssetId {
				policy_id: REWARDS_TOKEN_POLICY_ID,
				asset_name: AssetName::from_hex_unsafe(REWARDS_TOKEN_ASSET_NAME_STR),
			},
			initial_deposit: INITIAL_DEPOSIT_AMOUNT,
		},
		genesis_utxo,
		&governance_authority_payment_key(),
		client,
		&FixedDelayRetries::new(Duration::from_millis(500), 100),
	)
	.await
	.unwrap()
}

async fn run_update_reserve_settings_management<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
>(
	genesis_utxo: UtxoId,
	updated_total_accrued_function_script_hash: PolicyId,
	client: &T,
) -> Option<McTxHash> {
	reserve::update_settings::update_reserve_settings(
		genesis_utxo,
		&governance_authority_payment_key(),
		updated_total_accrued_function_script_hash,
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
) {
	reserve::deposit::deposit_to_reserve(
		TokenAmount {
			token: AssetId {
				policy_id: REWARDS_TOKEN_POLICY_ID,
				asset_name: AssetName::from_hex_unsafe(REWARDS_TOKEN_ASSET_NAME_STR),
			},
			amount: DEPOSIT_AMOUNT,
		},
		genesis_utxo,
		&governance_authority_payment_key(),
		client,
		&FixedDelayRetries::new(Duration::from_millis(500), 100),
	)
	.await
	.unwrap();
}

async fn run_handover_reserve<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
>(
	genesis_utxo: UtxoId,
	client: &T,
) -> Result<McTxHash, anyhow::Error> {
	reserve::handover::handover_reserve(
		genesis_utxo,
		&governance_authority_payment_key(),
		client,
		&FixedDelayRetries::new(Duration::from_millis(500), 100),
	)
	.await
}

async fn run_release_reserve_funds<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
>(
	genesis_utxo: UtxoId,
	release_amount: u64,
	reference_utxo: UtxoId,
	client: &T,
) {
	release_reserve_funds(
		TokenAmount {
			token: AssetId {
				policy_id: REWARDS_TOKEN_POLICY_ID,
				asset_name: AssetName::from_hex_unsafe(REWARDS_TOKEN_ASSET_NAME_STR),
			},
			amount: release_amount,
		},
		genesis_utxo,
		reference_utxo,
		&governance_authority_payment_key(),
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
					signature: MainchainSignature([19u8; 64]),
				},
				partner_chain_pub_key: SidechainPublicKey([20u8; 32].to_vec()),
				partner_chain_signature: partnerchain_signature,
				own_pkh: EVE_PUBLIC_KEY_HASH,
				registration_utxo,
				aura_pub_key: AuraPublicKey([22u8; 32].to_vec()),
				grandpa_pub_key: GrandpaPublicKey([23u8; 32].to_vec()),
			},
			&eve_payment_key(),
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
	let token_amount_at_illiquid_supply = utxos
		.into_iter()
		.flat_map(|utxo| {
			utxo.value
				.native_tokens
				.get(&token_policy_id.0)
				.and_then(|assets| assets.iter().find(|a| a.name == token_asset_name.0.to_vec()))
				.map(|asset| asset.amount as u64)
		})
		.sum::<u64>();
	assert_eq!(
		token_amount_at_illiquid_supply,
		expected_amount,
		"Expected {expected_amount} of {}.{} at {}, found {token_amount_at_illiquid_supply}",
		hex::encode(token_policy_id.0),
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

async fn assert_mutable_settings_eq<T: QueryLedgerState + ogmios_client::OgmiosClient>(
	genesis_utxo: UtxoId,

	updated_total_accrued_function_script_hash: PolicyId,
	client: &T,
) {
	let reserve_datum = get_reserve_datum(genesis_utxo, client).await;

	let mutable_settings = reserve_datum.mutable_settings;
	assert_eq!(
		mutable_settings.total_accrued_function_script_hash,
		updated_total_accrued_function_script_hash
	);
	assert_eq!(mutable_settings.initial_incentive, 0);
}

async fn get_reserve_datum<
	T: QueryLedgerState + ogmios_client::OgmiosClient + ogmios_client::query_network::QueryNetwork,
>(
	genesis_utxo: UtxoId,
	client: &T,
) -> ReserveDatum {
	let scripts_data =
		scripts_data::get_scripts_data_with_ogmios(genesis_utxo, client).await.unwrap();
	let validator_address = scripts_data.addresses.reserve_validator;
	let validator_utxos = client.query_utxos(&[validator_address]).await.unwrap();

	validator_utxos
		.into_iter()
		.find_map(|utxo| {
			let reserve_auth_policy_id = scripts_data.policy_ids.reserve_auth.0;
			let reserve_auth_asset_name: Vec<u8> = Vec::new();
			let auth_token =
				utxo.value.native_tokens.get(&reserve_auth_policy_id).and_then(|assets| {
					assets
						.iter()
						.find(|asset| asset.name == reserve_auth_asset_name && asset.amount == 1u64)
				});
			auth_token?;
			utxo.clone()
				.datum
				.and_then(|d| cardano_serialization_lib::PlutusData::from_bytes(d.bytes).ok())
				.and_then(|d| ReserveDatum::try_from(d).ok())
		})
		.unwrap()
}

async fn assert_reserve_handed_over<T: QueryLedgerState>(
	genesis_utxo: UtxoId,
	amount: u64,
	client: &T,
) {
	let data = scripts_data::get_scripts_data(genesis_utxo, NetworkIdKind::Testnet).unwrap();
	assert_token_amount_eq(
		&data.addresses.illiquid_circulation_supply_validator,
		&REWARDS_TOKEN_POLICY_ID,
		&AssetName::from_hex_unsafe(REWARDS_TOKEN_ASSET_NAME_STR),
		amount,
		client,
	)
	.await;
}

async fn assert_illiquid_supply<T: QueryLedgerState>(
	genesis_utxo: UtxoId,
	amount: u64,
	client: &T,
) {
	let data = scripts_data::get_scripts_data(genesis_utxo, NetworkIdKind::Testnet).unwrap();
	assert_token_amount_eq(
		&data.addresses.illiquid_circulation_supply_validator,
		&REWARDS_TOKEN_POLICY_ID,
		&AssetName::from_hex_unsafe(REWARDS_TOKEN_ASSET_NAME_STR),
		amount,
		client,
	)
	.await;
}
