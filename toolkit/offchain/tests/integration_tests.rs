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
	reserve::{self, release::release_reserve_funds},
	scripts_data, update_governance,
};
use partner_chains_plutus_data::reserve::ReserveDatum;
use sidechain_domain::{
	AdaBasedStaking, AssetId, AssetName, AuraPublicKey, CandidateRegistration, DParameter,
	GrandpaPublicKey, MainchainKeyHash, MainchainSignature, McTxHash, PermissionedCandidateData,
	PolicyId, SidechainPublicKey, SidechainSignature, StakePoolPublicKey, UtxoId, UtxoIndex,
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

const EVE_PUBLIC_KEY: StakePoolPublicKey =
	StakePoolPublicKey(hex!("a5ab6e82531cac3480cf7ff360f38a0beeea93cabfdd1ed0495e0423f7875c57"));

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
	run_release_reserve_funds(genesis_utxo, RELEASE_AMOUNT, V_FUNCTION_UTXO, &client).await;
	assert_reserve_deposited(
		genesis_utxo,
		INITIAL_DEPOSIT_AMOUNT + DEPOSIT_AMOUNT - 2 * RELEASE_AMOUNT,
		&client,
	)
	.await;
	assert_illiquid_supply(genesis_utxo, 2 * RELEASE_AMOUNT, &client).await;
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
async fn reserve_release_to_zero_scenario() {
	let image = GenericImage::new(TEST_IMAGE, TEST_IMAGE_TAG);
	let client = Cli::default();
	let container = client.run(image);
	let client = initialize(&container).await;
	let genesis_utxo = run_init_goveranance(&client).await;
	let txs = run_init_reserve_management(genesis_utxo, &client).await;
	assert_eq!(txs.len(), 3);
	let _ = run_create_reserve_management(genesis_utxo, V_FUNCTION_HASH, &client).await;
	assert_reserve_deposited(genesis_utxo, INITIAL_DEPOSIT_AMOUNT, &client).await;
	run_release_reserve_funds(genesis_utxo, INITIAL_DEPOSIT_AMOUNT, V_FUNCTION_UTXO, &client).await;
	assert_reserve_deposited(genesis_utxo, 0, &client).await;
	assert_illiquid_supply(genesis_utxo, INITIAL_DEPOSIT_AMOUNT, &client).await;
	run_handover_reserve(genesis_utxo, &client).await.unwrap();
	assert_reserve_handed_over(genesis_utxo, INITIAL_DEPOSIT_AMOUNT, &client).await;
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

// Proves that our code can still understand Plutus Script MultiSig from PCSC
#[tokio::test]
async fn update_legacy_governance() {
	let image = GenericImage::new(TEST_IMAGE, TEST_IMAGE_TAG);
	let client = Cli::default();
	let container = client.run(image);
	let client = initialize(&container).await;
	let legacy_init_governance_tx = hex!("84a900d9010281825820f8fbe7316561e57de9ecd1c86ee8f8b512a314ba86499ba9a584bfa8fe2edc8d000182a400581d707eccc25232d07fdc848d7b653465fc6c23a32d8dfb266181ecd26f0f01821a0032a3aca1581cab81fe48f392989bd215f9fdc25ece3335a248696b2a64abc1acb595a14e56657273696f6e206f7261636c6501028201d81858229f1820581cab81fe48f392989bd215f9fdc25ece3335a248696b2a64abc1acb595ff03d8185901db82025901d65901d30100003323322323232323322323232222323232532323355333573466e20cc8c8c88c008004c058894cd4004400c884cc018008c010004c04488004c04088008c01000400840304034403c4c02d24010350543500300d37586ae84008dd69aba1357440026eb0014c040894cd400440448c884c8cd40514cd4c00cc04cc030dd6198009a9803998009a980380411000a40004400290080a400429000180300119112999ab9a33710002900009807a490350543600133003001002301522253350011300f49103505437002215333573466e1d20000041002133005337020089001000980991299a8008806910a999ab9a3371e00a6eb800840404c0100048c8cc8848cc00400c008d55ce80098031aab9e00137540026016446666aae7c00480348cd4030d5d080118019aba2002498c02888cccd55cf8009006119a8059aba100230033574400493119319ab9c00100512200212200130062233335573e0024010466a00e6eb8d5d080118019aba20020031200123300122337000040029000180191299a800880211099a802801180200089100109109119800802001919180080091198019801001000a61239f9f581ce8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2bff01ff000182581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b1a3b61e903021a00063d5109a1581cab81fe48f392989bd215f9fdc25ece3335a248696b2a64abc1acb595a14e56657273696f6e206f7261636c65010b5820bb4035b9ede213192640b6e68ddea7d6c42ad664a9b4d1fbff04b52193cec1ae0dd9010281825820f8fbe7316561e57de9ecd1c86ee8f8b512a314ba86499ba9a584bfa8fe2edc8d040ed9010281581ce8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b1082581d60e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b1b006a8e81df3a2cc6111a00095bfaa300d9010281825820fc014cb5f071f5d6a36cb5a7e5f168c86555989445a23d4abec33d280f71aca4584089d9dfaddfa820868d23f27a8344c711507b95c117c76862a191c2484ba364814db7cc196f2b98de4f40ac6069effa115c2d4f9781744ef3cc4f141e82c3e40c06d9010281590cc2590cbf0100003332323233223232323232323233223232323232323232323233223232323232323232232323232322225335323232323233353232325333573466e1d200000213322122233002005004375a6ae84004dd71aba1357440022a666ae68cdc3a40040042664424446600200a0086eb4d5d08009bae357426ae8800454ccd5cd19b87480100084c84888c00c010dd69aba1001130314901035054310035573c0046aae74004dd50039191919299a998082481174552524f522d56455253494f4e2d504f4c4943592d3037003303422533500110332213235003223500122225333500210072153500522350172233532335005233500425333573466e3c0080045400c40b880b88cd401080b894ccd5cd19b8f00200115003102e153350032153350022133500223350022335002233500223303000200120312335002203123303000200122203122233500420312225333573466e1c01800c54ccd5cd19b8700500213302c00400110331033102c153350012102c102c133044225335001100e22132533500321350012253353302c00201c153353302c333027010502048810e56657273696f6e206f7261636c6500480084cd41200d0010401040104004c010004c8cd4104004108c094014403084020c010004c080c07cc04cdd619801180091000a40002a66a660229201174552524f522d56455253494f4e2d504f4c4943592d3038005335330342253350011033221325333573466e24ccc050c078cc01cd4c0c800c880052002500d4890e56657273696f6e206f7261636c65004800040044cd40d40e0004c010004c0a4c04cdd619801180091000a4008203844203a266022921174552524f522d56455253494f4e2d504f4c4943592d3039003300e500800a101b101b5302c002502a50042232533533010491174552524f522d56455253494f4e2d504f4c4943592d303100300c302a302930123758660026a6058660026a605801244002900011000a40002a66a6601e9201174552524f522d56455253494f4e2d504f4c4943592d30320033005003002133010491174552524f522d56455253494f4e2d504f4c4943592d3033005004101a101a502a2253353300e491174552524f522d56455253494f4e2d504f4c4943592d30340033004002001153353300f491174552524f522d56455253494f4e2d504f4c4943592d3035003300c500600813300f4901174552524f522d56455253494f4e2d504f4c4943592d30360050031019101913300f33300a30143233502835302900122001480214009400d2210e56657273696f6e206f7261636c65004800888cc0c0894cd400440bc884c8d400c88894ccd40084014854cd4008854d401888d404888cd4c8cd40148cd401094ccd5cd19b8f00200115003102920292335004202925333573466e3c0080045400c40a454cd400c854cd400884cd40088cd40088cd40088cd40088cc0ac00800480b08cd400880b08cc0ac0080048880b0888cd401080b08894ccd5cd19b8700600315333573466e1c0140084cc09c01000440b840b8409c54cd40048409c409c4cc0fc894cd40044034884c94cd400c84d4004894cd4cc09c00806454cd4cc0b403406054cd4cc09cccc088045406d2210e56657273696f6e206f7261636c6500480084cd410c0bc0104010401040104004c010004c8cd40f00040f4c080018402c401884018c010004c068c8c068c040dd619a8149a981500091000a4008a006266a04a6a604c0064400290000a99aa99a9a981299a8121a981280111000a400444a66a0022a042442a66a0022a666ae68cdc3a4000008260480042a046442a04a4260426eb80045407c840044c0a92401164552524f522d4f5241434c452d504f4c4943592d313000301a003102a1302949010350543500302722533500110102215333573466ebc024008404c4c01000488c8c8c94cd4c94cd4c8cc0b4894cd400454088884d4008894ccd5cd19b8f002007130270011300600300253353302c225335001102b2213235003223500122225333500210072153350022133039225335001100b2213253350032135001225335330210024810054ccd5cd19b8748008ccc07003406d2210e56657273696f6e206f7261636c6500133503d009004100410041001300400132335036001037301a0021008210083004001300e3232323302f225335001100322133502f00230040010023012300d37586600c6004440029000180818061bac33005300122001480094c094cc010cc098800400920001302a491194552524f522d56455253494f4e2d43555252454e43592d30310022153350011002221302e491194552524f522d56455253494f4e2d43555252454e43592d3031002130210011501f3233301e75ca03a002660066a6048660066604a4002002900011000a401042a66a00220264426a00444a66a0062666ae68cdc4800a400002e03044203220246aae78004dd50012810111191981491299a8008a40004426a00444a666ae68cdc78010048980380089803001802181411299a8008a40004426a00444a666ae68cdc780100388008980300191299a80089812001110a99a8008801110981400311299a8008806899ab9c00200c30212233335573e0024042466a0406ae84008c00cd5d100124c44666ae68cdc380100080500491999999aba4001250142501423232333002375800800244a6646aa66a6666660020064400c400a400a46036002400a42603600220084266600c00600a44a66a666666008004440124010401040104603c00242666012004603c246600200a00444014200e4444446666666ae900188c8cc01cd55ce8009aab9e001375400e4600a6eac01c8c010dd6003918019bad00723002375c00e0542006a02a4446666aae7c00c800c8cc008d5d08021aba2004023250142501401f301e225335001101d22133501e300f0023004001301d225335001101c22133501d0023004001301c225335001101b22133501c0023004001233300e75ca01a00244666ae68cdc7801000802001891001091000980b91299a800880b11099a80b8011802000980b11299a800880a91099a80b18040011802000980a91299a800880a11099a80a8011802000980a11299a800880991099a80a1802801180200091919192999ab9a3370e90000010999109198008018011919192999ab9a3370e90000010999109198008018011919192999ab9a3370e900000109bae35742002260369201035054310035573c0046aae74004dd51aba1001375a6ae84d5d10008980c2481035054310035573c0046aae74004dd51aba10013005357426ae880044c055241035054310035573c0046aae74004dd500091919192999ab9a3370e9000001099191999911109199980080280200180118039aba100333300a75ca0126ae84008c8c8c94ccd5cd19b87480000084488800c54ccd5cd19b87480080084c84888c004010dd71aba100115333573466e1d20040021321222300200435742002260329201035054310035573c0046aae74004dd51aba10013300875c6ae84d5d10009aba200135744002260289201035054310035573c0046aae74004dd500091919192999ab9a3370e90000010991991091980080180118009aba10023300623232325333573466e1d200000213212230020033005357420022a666ae68cdc3a400400426466644424466600200a0080066eb4d5d08011bad357420026eb4d5d09aba200135744002260309201035054310035573c0046aae74004dd50009aba1357440044646464a666ae68cdc3a400000426424460040066eb8d5d08008a999ab9a3370e900100109909118008019bae357420022602e921035054310035573c0046aae74004dd500089809a49035054310035573c0046aae74004dd5000911919192999ab9a3370e90010010a8040a999ab9a3370e90000010980498029aba1001130134901035054310035573c0046aae74004dd5000899800bae75a4464460046eac004c04088cccd55cf800900811919a8081980798031aab9d001300535573c00260086ae8800cd5d0801008909118010018891000980591299a800880511099a8058011802000980511299a800880491099a8050011802000980491299a800880411099a80499a8029a980300111000a4000600800226444a666ae68cdc4000a4000260129210350543600133003001002300822253350011300949103505437002215333573466e1d20000041002133005337020089001000919198021aab9d0013233004200100135573c0026ea80048c88c008004c01c88cccd55cf8009003919a80318021aba10023003357440049311091980080180109100109109119800802001919319ab9c001002120012323001001223300330020020014c012bd8799fd8799f5820f8fbe7316561e57de9ecd1c86ee8f8b512a314ba86499ba9a584bfa8fe2edc8dff00ff004c0129d8799fd87a9f581c7eccc25232d07fdc848d7b653465fc6c23a32d8dfb266181ecd26f0fffd87a80ff000105a182010082d8799f1820581c08b95138e16a062fa8d623a2b1beebd59c06210f3d33690580733e73ff821a000c0cfa1a0db12739f5f6");
	let result = client.submit_transaction(&legacy_init_governance_tx).await.unwrap();
	let await_tx = &FixedDelayRetries::new(Duration::from_millis(500), 100);
	await_tx
		.await_tx_output(&client, UtxoId::new(result.transaction.id, 0))
		.await
		.unwrap();
	let genesis_utxo =
		UtxoId::new(hex!("f8fbe7316561e57de9ecd1c86ee8f8b512a314ba86499ba9a584bfa8fe2edc8d"), 0);
	assert!(run_upsert_d_param(genesis_utxo, 0, 1, &governance_authority_payment_key(), &client)
		.await
		.is_some());
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
		DEPOSIT_AMOUNT,
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
		release_amount.try_into().unwrap(),
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
