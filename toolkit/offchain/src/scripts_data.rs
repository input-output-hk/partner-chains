use crate::{
	csl::{ogmios_network_to_csl, plutus_script_address, plutus_script_hash},
	untyped_plutus::{apply_params_to_script, csl_plutus_data_to_uplc, datum_to_uplc_plutus_data},
	OffchainError,
};
use anyhow::anyhow;
use cardano_serialization_lib::{LanguageKind, NetworkIdKind, PlutusData as CSLPlutusData};
use chain_params::SidechainParams;
use ogmios_client::query_network::QueryNetwork;
use plutus::ToDatum;
use serde::Serialize;
use sidechain_domain::{MainchainAddressHash, PolicyId};
use uplc::PlutusData;

/// Provides convenient access to the addresses and hashes of the partner chain smart contracts.
/// Data in this struct is derived from the smart contracts, applied parameters and the network.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptsData {
	pub addresses: Addresses,
	pub validator_hashes: ValidatorHashes,
	pub policy_ids: PolicyIds,
}

/// Bech32 address of applied validators in partner-chains smart contracts.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Addresses {
	pub committee_candidate_validator: String,
	pub d_parameter_validator: String,
	pub illiquid_circulation_supply_validator: String,
	pub permissioned_candidates_validator: String,
	pub reserve_validator: String,
	pub version_oracle_validator: String,
}

/// Hashes of applied validators in partner-chains smart contracts.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ValidatorHashes {
	pub committee_candidate_validator: MainchainAddressHash,
	pub d_parameter_validator: MainchainAddressHash,
	pub illiquid_circulation_supply_validator: MainchainAddressHash,
	pub permissioned_candidates_validator: MainchainAddressHash,
	pub reserve_validator: MainchainAddressHash,
	pub version_oracle_validator: MainchainAddressHash,
}

/// Policy IDs of applied scripts in partner-chains smart contracts.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct PolicyIds {
	pub d_parameter: PolicyId,
	pub init_token: PolicyId,
	pub governance: PolicyId,
	pub permissioned_candidates: PolicyId,
	pub reserve_auth: PolicyId,
	pub version_oracle: PolicyId,
}

type ScriptBytes = Vec<u8>;

pub trait GetScriptsData {
	#[allow(async_fn_in_trait)]
	/// For the given `pc_params` it returns the addresses, hashes and policy ids of the partner chain smart contracts.
	async fn get_scripts_data(
		&self,
		pc_params: SidechainParams,
	) -> Result<ScriptsData, OffchainError>;
}

impl<T: QueryNetwork> GetScriptsData for T {
	async fn get_scripts_data(
		&self,
		pc_params: SidechainParams,
	) -> Result<ScriptsData, OffchainError> {
		let network = ogmios_network_to_csl(
			self.shelley_genesis_configuration()
				.await
				.map_err(|e| OffchainError::OgmiosError(e.to_string()))?
				.network,
		);
		get_scripts_data(pc_params, network)
			.map_err(|e| OffchainError::InternalError(e.to_string()))
	}
}

fn get_scripts_data(
	pc_params: SidechainParams,
	network: NetworkIdKind,
) -> anyhow::Result<ScriptsData> {
	let pc_params_data = datum_to_uplc_plutus_data(&pc_params.to_datum());

	let (version_oracle_validator, version_oracle_policy, version_oracle_policy_data) =
		version_oracle(&pc_params_data, network)?;

	let committee_candidate_validator =
		apply_params_to_script(&[&pc_params_data], raw_scripts::COMMITTEE_CANDIDATE_VALIDATOR)?;
	let d_parameter_validator =
		apply_params_to_script(&[&pc_params_data], raw_scripts::D_PARAMETER_VALIDATOR)?;
	let illiquid_circulation_supply_validator = apply_params_to_script(
		&[&version_oracle_policy_data],
		raw_scripts::ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR,
	)?;
	let permissioned_candidates_validator =
		apply_params_to_script(&[&pc_params_data], raw_scripts::PERMISSIONED_CANDIDATES_VALIDATOR)?;
	let reserve_validator =
		apply_params_to_script(&[&version_oracle_policy_data], raw_scripts::RESERVE_VALIDATOR)?;

	Ok(ScriptsData {
		addresses: Addresses {
			committee_candidate_validator: to_bech32(&committee_candidate_validator, network)?,
			d_parameter_validator: to_bech32(&d_parameter_validator, network)?,
			illiquid_circulation_supply_validator: to_bech32(
				&illiquid_circulation_supply_validator,
				network,
			)?,
			permissioned_candidates_validator: to_bech32(
				&permissioned_candidates_validator,
				network,
			)?,
			reserve_validator: to_bech32(&reserve_validator, network)?,
			version_oracle_validator: to_bech32(&version_oracle_validator, network)?,
		},
		validator_hashes: ValidatorHashes {
			committee_candidate_validator: mc_hash(&committee_candidate_validator),
			d_parameter_validator: mc_hash(&d_parameter_validator),
			illiquid_circulation_supply_validator: mc_hash(&illiquid_circulation_supply_validator),
			permissioned_candidates_validator: mc_hash(&permissioned_candidates_validator),
			reserve_validator: mc_hash(&reserve_validator),
			version_oracle_validator: mc_hash(&version_oracle_validator),
		},
		policy_ids: PolicyIds {
			d_parameter: get_script_policy_id(
				raw_scripts::D_PARAMETER_POLICY,
				&[&pc_params_data, &plutus_script_to_data(&d_parameter_validator, network)?],
			)?,
			init_token: get_script_policy_id(raw_scripts::INIT_TOKEN_POLICY, &[&pc_params_data])?,
			governance: get_script_policy_id(
				raw_scripts::MULTI_SIG_POLICY,
				&[&multisig_governance_policy_configuration(pc_params)],
			)?,
			permissioned_candidates: get_script_policy_id(
				raw_scripts::PERMISSIONED_CANDIDATES_POLICY,
				&[
					&pc_params_data,
					&plutus_script_to_data(&permissioned_candidates_validator, network)?,
				],
			)?,
			reserve_auth: get_script_policy_id(
				raw_scripts::RESERVE_AUTH_POLICY,
				&[&version_oracle_policy_data],
			)?,
			version_oracle: version_oracle_policy,
		},
	})
}

// Returns version oracle script, policy and PlutusData required by other scripts.
fn version_oracle(
	pc_params_data: &PlutusData,
	network: NetworkIdKind,
) -> Result<(ScriptBytes, PolicyId, PlutusData), anyhow::Error> {
	let init_token_policy =
		get_script_policy_id(raw_scripts::INIT_TOKEN_POLICY, &[pc_params_data])?;
	let init_token_asset_data = PlutusData::Array(vec![
		PlutusData::BoundedBytes(init_token_policy.0.to_vec().into()),
		PlutusData::BoundedBytes(b"Version oracle InitToken".to_vec().into()),
	]);

	let validator =
		apply_params_to_script(&[pc_params_data], raw_scripts::VERSION_ORACLE_VALIDATOR)?;
	let validator_address = plutus_script_address(&validator, network, LanguageKind::PlutusV2);
	let policy_script = apply_params_to_script(
		&[
			pc_params_data,
			&init_token_asset_data,
			&csl_plutus_data_to_uplc(&CSLPlutusData::from_address(&validator_address)?)?,
		],
		raw_scripts::VERSION_ORACLE_POLICY,
	)?;
	let policy = PolicyId(plutus_script_hash(&policy_script, LanguageKind::PlutusV2));
	let policy_data = PlutusData::BoundedBytes(policy.0.to_vec().into());
	Ok((validator, policy, policy_data))
}

// Applies parameters to the script and returns its hash.
fn get_script_policy_id(policy_bytes: &[u8], params: &[&PlutusData]) -> anyhow::Result<PolicyId> {
	let policy_script = apply_params_to_script(params, policy_bytes)?;
	Ok(PolicyId(plutus_script_hash(&policy_script, LanguageKind::PlutusV2)))
}

// Returns PlutusData representation of the given script. It is done in the same way as on-chain code expects.
// First, the Address is created, then it is converted to PlutusData.
fn plutus_script_to_data(
	script_bytes: &[u8],
	network: NetworkIdKind,
) -> anyhow::Result<PlutusData> {
	let validator_address = plutus_script_address(script_bytes, network, LanguageKind::PlutusV2);
	// Address is CSL, it has to be converted to UPLC, in order to apply it to the script.
	csl_plutus_data_to_uplc(&CSLPlutusData::from_address(&validator_address)?)
}

// Returns the simplest MultiSig policy configuration plutus data:
// there is one required authority and it is the governance authority from sidechain params.
fn multisig_governance_policy_configuration(params: SidechainParams) -> PlutusData {
	PlutusData::Array(vec![
		PlutusData::Array(vec![uplc::PlutusData::BoundedBytes(
			params.governance_authority.0.to_vec().into(),
		)]),
		PlutusData::BigInt(uplc::BigInt::Int(1.into())),
	])
}

// Returns bech32 address of the given PlutusV2 script
fn to_bech32(script_bytes: &[u8], network: NetworkIdKind) -> anyhow::Result<String> {
	let address = plutus_script_address(script_bytes, network, LanguageKind::PlutusV2);
	address.to_bech32(None).map_err(|e| anyhow!(e))
}

fn mc_hash(script_bytes: &[u8]) -> MainchainAddressHash {
	let bytes = plutus_script_hash(script_bytes, LanguageKind::PlutusV2);
	MainchainAddressHash(bytes)
}

#[cfg(test)]
mod tests {
	use crate::scripts_data::{Addresses, PolicyIds, ScriptsData, ValidatorHashes};
	use cardano_serialization_lib::NetworkIdKind;
	use chain_params::SidechainParams;
	use hex_literal::hex;
	use sidechain_domain::{MainchainAddressHash, McTxHash, PolicyId, UtxoId};

	pub(crate) const TEST_PARAMS: SidechainParams = SidechainParams {
		chain_id: 0,
		threshold_numerator: 2,
		threshold_denominator: 3,
		genesis_committee_utxo: UtxoId {
			tx_hash: McTxHash(hex!(
				"0000000000000000000000000000000000000000000000000000000000000000"
			)),
			index: sidechain_domain::UtxoIndex(0),
		},
		governance_authority: MainchainAddressHash(hex!(
			"044741127bce1895f93fc8ac7bedb8930066b3c3964db00518bd2ccc"
		)),
	};

	pub(crate) fn scripts_data_test_vector() -> ScriptsData {
		ScriptsData {
			addresses: Addresses {
				committee_candidate_validator:
					"addr_test1wq8vwhqkfyrz0qu3sf89qdvaj8slrjlwwhlyzw9ayv0rj5qz3ne4t".into(),
				d_parameter_validator:
					"addr_test1wp3cw59x95h6x0mcuaydznhxpagdvnhaj9zsdavrkvjfhxg2rctxe".into(),
				illiquid_circulation_supply_validator:
					"addr_test1wp5ahp39vsw03kx7tze2sdzd5f5twfts46tzdkdjrhnj94g9xwsus".into(),
				permissioned_candidates_validator:
					"addr_test1wqvxznvu7f9u8svs5sa0rxg3ggvcz4us5w8k63r70yztntqpqpwxx".into(),
				reserve_validator:
					"addr_test1wzc6vvzqd9x4fvx09xgep4sna20skxj86p6yan9ymv9q99sa8xd97".into(),
				version_oracle_validator:
					"addr_test1wzgctwxcrr555ej8rqrdjkjlfmv47ck2vltcp9enz2s0z8qad37fs".into(),
			},
			validator_hashes: ValidatorHashes {
				committee_candidate_validator: MainchainAddressHash(hex!(
					"0ec75c164906278391824e50359d91e1f1cbee75fe4138bd231e3950"
				)),
				d_parameter_validator: MainchainAddressHash(hex!(
					"638750a62d2fa33f78e748d14ee60f50d64efd914506f583b3249b99"
				)),
				illiquid_circulation_supply_validator: MainchainAddressHash(hex!(
					"69db8625641cf8d8de58b2a8344da268b72570ae9626d9b21de722d5"
				)),
				permissioned_candidates_validator: MainchainAddressHash(hex!(
					"18614d9cf24bc3c190a43af199114219815790a38f6d447e7904b9ac"
				)),
				reserve_validator: MainchainAddressHash(hex!(
					"b1a63040694d54b0cf299190d613ea9f0b1a47d0744ecca4db0a0296"
				)),
				version_oracle_validator: MainchainAddressHash(hex!(
					"9185b8d818e94a66471806d95a5f4ed95f62ca67d780973312a0f11c"
				)),
			},
			policy_ids: PolicyIds {
				d_parameter: PolicyId(hex!(
					"88b9d9798dde404b4ff488be62de3d7744a2e282934b5bd1687d8cbe"
				)),
				init_token: PolicyId(hex!(
					"ae91bce3634bbf0d2748cb2c9b5a4cd547da3fabb99d33e762c87704"
				)),
				governance: PolicyId(hex!(
					"7d55e8e2f8f0637d6ab99975d8ab9b6112976eec1e778d3f770fe102"
				)),
				permissioned_candidates: PolicyId(hex!(
					"f7449038957139c2782d81fc72bc889898fd24d20059535a681b3774"
				)),
				reserve_auth: PolicyId(hex!(
					"22dc2777f1d73504a2d9db99067b82ffe4abddc31ed1f9f6d97ca7d7"
				)),
				version_oracle: PolicyId(hex!(
					"4d982fae61319220e75e29054bad955484faa24dba65046136b8e6cb"
				)),
			},
		}
	}

	// Expected values are the ones obtained from pc-contracts-cli for the TEST_PARAMS.
	#[test]
	fn test_get_scripts_data() {
		let actual =
			crate::scripts_data::get_scripts_data(TEST_PARAMS, NetworkIdKind::Testnet).unwrap();
		assert_eq!(scripts_data_test_vector(), actual);
	}
}
