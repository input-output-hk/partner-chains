use crate::chain_spec::get_account_id_from_seed;
use crate::chain_spec::*;
use chain_params::SidechainParams;
use sc_service::ChainType;
use sidechain_domain::*;
use sidechain_runtime::{
	AccountId, AuraConfig, BalancesConfig, GrandpaConfig, NativeTokenManagementConfig,
	RuntimeGenesisConfig, SessionCommitteeManagementConfig, SessionConfig, SidechainConfig,
	SudoConfig, SystemConfig,
};
use sp_core::bytes::from_hex;
use sp_core::{ed25519, sr25519};
use std::str::FromStr;

pub fn authority_keys(
	aura_pub_key: &str,
	grandpa_pub_key: &str,
	sidechain_pub_key: &str,
) -> AuthorityKeys {
	let aura_pk = sr25519::Public::from_raw(from_hex(aura_pub_key).unwrap().try_into().unwrap());
	let granda_pk =
		ed25519::Public::from_raw(from_hex(grandpa_pub_key).unwrap().try_into().unwrap());
	let sidechain_pk = sidechain_domain::SidechainPublicKey(from_hex(sidechain_pub_key).unwrap());

	let session_keys = (aura_pk, granda_pk).into();
	AuthorityKeys { session: session_keys, cross_chain: sidechain_pk.try_into().unwrap() }
}

pub fn staging_initial_authorities() -> Vec<AuthorityKeys> {
	vec![
		// validator-1
		authority_keys(
			"0xba94651de6279a38a416b97b9720c3df76224435e951ac73e9e302a4ee9fcf73",
			"0xdde2501588713ddad5daf5a898c19d82cd591609c9184679868640c8cfe8287d",
			"0x03b827f4da9711bab7292e5695576a841a4d20af9a07b1ba7a230168d2a78e9df4",
		),
		// validator-2
		authority_keys(
			"0x36128fff2acc04f206ccaf4e9f8e9995998efced29075a58b7d76d3735c21208",
			"0x8f9a9856a27cc114ce85b64f41144f0c907c4bd8b3102b083b52b6b61aff6c47",
			"0x02ef5bcd94d54a18ad199559782cd72ac3ccd850976aaaafbca8f9d2625afbf7c4",
		),
		// validator-3
		authority_keys(
			"0x9a32d3896a56e822321f7bc915befc8ce112c5d67e3c6497295bd3d7b020f94c",
			"0x4f3c0ecc6dc474f27ad7967f5cdbd50da047ffedbc91b65f5cd247515489c98f",
			"0x02f2762ab6e1a125dc03908a7b738f8023d13763f28a11d7633c6c8bc463478430",
		),
		// validator-4
		authority_keys(
			"0xc41992b8eb2f3a8a6c46211df584827f9eeb0175e2c75e1242392262b55b6874",
			"0x34b71fdad96431bf115350d8ad21eec07a2b154ff32dc31125f988e308bebea8",
			"0x025e19f82c5e2bac5e8869d49ff26359e442628bc5cfa38eeb5275f43d04015da8",
		),
		// validator-5
		authority_keys(
			"0x500d7ff6d903c85db5ee5624df9510c2a085cf30da260166bd370010d0bdc97a",
			"0xa04d74c1539550876d04e4d2de4e0531087c3b6810ce96ddc16d78ccf4ac4f11",
			"0x03f38a062a4b372c045c1dddc4fe98a2c9cb1d6eec8bf02f973fd29b1096cd8155",
		),
		// validator-6
		authority_keys(
			"0xc2a8354c3928ffacf21eb8b2c3e6bceda9d54b0e2ce10f48fe8b9afafb7d8a3a",
			"0x82e56b009c755e4f8a4dcb2ff22d1e6b98d984b4df02d83a75157335222b218b",
			"0x033d3a2e581821fdd222581f6015eaabc798dd4dc0f7eeb3d6630b84449d76c9c9",
		),
		// validator-7
		authority_keys(
			"0x22c9f9d51022b7ad2204131e6268ab079c84bcdb44a4c6907affed5779da9c7b",
			"0x5d100e44ecd41aeb0292d17bdefb99ebfc879682a1cd8b489ed0a66d3ee0b391",
			"0x0232ebed4c0c742fa951b471fe6f6f2f09a2d235bf7e9992fbf786cf032c97247e",
		),
	]
}
pub fn staging_endowed_accounts() -> Vec<AccountId> {
	vec![
		AccountId::from_str("0xba94651de6279a38a416b97b9720c3df76224435e951ac73e9e302a4ee9fcf73")
			.unwrap(),
		AccountId::from_str("0x36128fff2acc04f206ccaf4e9f8e9995998efced29075a58b7d76d3735c21208")
			.unwrap(),
		AccountId::from_str("0x9a32d3896a56e822321f7bc915befc8ce112c5d67e3c6497295bd3d7b020f94c")
			.unwrap(),
		AccountId::from_str("0xc41992b8eb2f3a8a6c46211df584827f9eeb0175e2c75e1242392262b55b6874")
			.unwrap(),
		AccountId::from_str("0x500d7ff6d903c85db5ee5624df9510c2a085cf30da260166bd370010d0bdc97a")
			.unwrap(),
		AccountId::from_str("0xc2a8354c3928ffacf21eb8b2c3e6bceda9d54b0e2ce10f48fe8b9afafb7d8a3a")
			.unwrap(),
		AccountId::from_str("0x22c9f9d51022b7ad2204131e6268ab079c84bcdb44a4c6907affed5779da9c7b")
			.unwrap(),
		staging_sudo_key(),
		// SDETs test accounts, keys are in https://github.com/input-output-hk/sidechains-tests/tree/master/secrets
		// negative-test
		AccountId::from_str("5F1N52dZx48UpXNLtcCzSMHZEroqQDuYKfidg46Tp37SjPcE").unwrap(),
		// faucet-0
		AccountId::from_str("5C7C2Z5sWbytvHpuLTvzKunnnRwQxft1jiqrLD5rhucQ5S9X").unwrap(),
	]
}

pub fn staging_sudo_key() -> AccountId {
	get_account_id_from_seed::<sr25519::Public>(
		"assist draw loud island six improve van gas slam urban penalty lyrics",
	)
}

pub fn staging_config() -> Result<ChainSpec, envy::Error> {
	Ok(ChainSpec::builder(runtime_wasm(), None)
		.with_name("Staging")
		.with_id("staging")
		.with_chain_type(ChainType::Local)
		.with_genesis_config(staging_genesis(
			// Initial PoA authorities
			staging_initial_authorities(),
			// Sudo account
			Some(staging_sudo_key()),
			// Pre-funded accounts
			staging_endowed_accounts(),
			true,
		)?)
		.build())
}

/// Configure initial storage state for FRAME modules.
pub fn staging_genesis(
	initial_authorities: Vec<AuthorityKeys>,
	root_key: Option<AccountId>,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> Result<serde_json::Value, envy::Error> {
	let config = RuntimeGenesisConfig {
		system: SystemConfig { ..Default::default() },
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
		},
		aura: AuraConfig { authorities: vec![] },
		grandpa: GrandpaConfig { authorities: vec![], ..Default::default() },
		sudo: SudoConfig {
			// Assign network admin rights.
			key: root_key,
		},
		transaction_payment: Default::default(),
		session: SessionConfig {
			initial_validators: initial_authorities
				.iter()
				.map(|authority_keys| {
					(authority_keys.cross_chain.clone().into(), authority_keys.session.clone())
				})
				.collect(),
		},
		sidechain: SidechainConfig {
			params: SidechainParams::read_from_env_with_defaults()?,
			..Default::default()
		},
		pallet_session: Default::default(),
		session_committee_management: SessionCommitteeManagementConfig {
			initial_authorities: initial_authorities
				.into_iter()
				.map(|keys| (keys.cross_chain, keys.session))
				.collect(),
			main_chain_scripts: sp_session_validator_management::MainChainScripts::read_from_env()?,
		},
		native_token_management: NativeTokenManagementConfig {
			main_chain_scripts: sp_native_token_management::MainChainScripts::read_from_env()?,
			..Default::default()
		},
	};

	Ok(serde_json::to_value(config).expect("Genesis config must be serialized correctly"))
}
