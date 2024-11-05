use crate::{csl::ogmios_network_to_csl, plutus_script::PlutusScript, OffchainError};
use cardano_serialization_lib::{LanguageKind::PlutusV2, NetworkIdKind};
use chain_params::SidechainParams;
use ogmios_client::query_network::QueryNetwork;
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

/// For the given `pc_params` it returns the addresses, hashes and policy ids of the partner chain smart contracts.
pub fn get_scripts_data(
	pc_params: SidechainParams,
	network: NetworkIdKind,
) -> anyhow::Result<ScriptsData> {
	let (version_oracle_validator, version_oracle_policy, version_oracle_policy_data) =
		version_oracle(pc_params.clone(), network)?;

	let committee_candidate_validator =
		PlutusScript::from_wrapped_cbor(raw_scripts::COMMITTEE_CANDIDATE_VALIDATOR, PlutusV2)?
			.apply_data(pc_params.clone())?;
	let d_parameter_validator =
		PlutusScript::from_wrapped_cbor(raw_scripts::D_PARAMETER_VALIDATOR, PlutusV2)?
			.apply_data(pc_params.clone())?;
	let illiquid_circulation_supply_validator = PlutusScript::from_wrapped_cbor(
		raw_scripts::ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR,
		PlutusV2,
	)?
	.apply_uplc_data(version_oracle_policy_data.clone())?;
	let permissioned_candidates_validator =
		PlutusScript::from_wrapped_cbor(raw_scripts::PERMISSIONED_CANDIDATES_VALIDATOR, PlutusV2)?
			.apply_data(pc_params.clone())?;
	let reserve_validator =
		PlutusScript::from_wrapped_cbor(raw_scripts::RESERVE_VALIDATOR, PlutusV2)?
			.apply_uplc_data(version_oracle_policy_data.clone())?;

	let d_parameter_policy =
		PlutusScript::from_wrapped_cbor(raw_scripts::D_PARAMETER_POLICY, PlutusV2)?
			.apply_data(pc_params.clone())?
			.apply_uplc_data(d_parameter_validator.plutus_address_data(network)?)?;
	let permissioned_candidates_policy =
		PlutusScript::from_wrapped_cbor(raw_scripts::PERMISSIONED_CANDIDATES_POLICY, PlutusV2)?
			.apply_data(pc_params.clone())?
			.apply_uplc_data(permissioned_candidates_validator.plutus_address_data(network)?)?;
	let governance_policy =
		PlutusScript::from_wrapped_cbor(raw_scripts::MULTI_SIG_POLICY, PlutusV2)?
			.apply_uplc_data(multisig_governance_policy_configuration(pc_params.clone()))?;
	let init_token_policy =
		PlutusScript::from_wrapped_cbor(raw_scripts::INIT_TOKEN_POLICY, PlutusV2)?
			.apply_data(pc_params.clone())?;
	let reserve_auth_policy =
		PlutusScript::from_wrapped_cbor(raw_scripts::RESERVE_AUTH_POLICY, PlutusV2)?
			.apply_uplc_data(version_oracle_policy_data)?;

	Ok(ScriptsData {
		addresses: Addresses {
			committee_candidate_validator: committee_candidate_validator
				.plutus_address_bech32(network)?,
			d_parameter_validator: d_parameter_validator.plutus_address_bech32(network)?,
			illiquid_circulation_supply_validator: illiquid_circulation_supply_validator
				.plutus_address_bech32(network)?,
			permissioned_candidates_validator: permissioned_candidates_validator
				.plutus_address_bech32(network)?,
			reserve_validator: reserve_validator.plutus_address_bech32(network)?,
			version_oracle_validator: version_oracle_validator.plutus_address_bech32(network)?,
		},
		validator_hashes: ValidatorHashes {
			committee_candidate_validator: committee_candidate_validator.script_plutus_address(),
			d_parameter_validator: d_parameter_validator.script_plutus_address(),
			illiquid_circulation_supply_validator: illiquid_circulation_supply_validator
				.script_plutus_address(),
			permissioned_candidates_validator: permissioned_candidates_validator
				.script_plutus_address(),
			reserve_validator: reserve_validator.script_plutus_address(),
			version_oracle_validator: version_oracle_validator.script_plutus_address(),
		},
		policy_ids: PolicyIds {
			d_parameter: d_parameter_policy.plutus_policy_id(),
			init_token: init_token_policy.plutus_policy_id(),
			governance: governance_policy.plutus_policy_id(),
			permissioned_candidates: permissioned_candidates_policy.plutus_policy_id(),
			reserve_auth: reserve_auth_policy.plutus_policy_id(),
			version_oracle: version_oracle_policy,
		},
	})
}

// Returns version oracle script, policy and PlutusData required by other scripts.
fn version_oracle(
	pc_params: SidechainParams,
	network: NetworkIdKind,
) -> Result<(PlutusScript, PolicyId, PlutusData), anyhow::Error> {
	let init_token_policy =
		PlutusScript::from_wrapped_cbor(raw_scripts::INIT_TOKEN_POLICY, PlutusV2)?
			.apply_data(pc_params.clone())?
			.plutus_policy_id();

	let init_token_asset_data = PlutusData::Array(vec![
		PlutusData::BoundedBytes(init_token_policy.0.to_vec().into()),
		PlutusData::BoundedBytes(b"Version oracle InitToken".to_vec().into()),
	]);

	let validator =
		PlutusScript::from_wrapped_cbor(raw_scripts::VERSION_ORACLE_VALIDATOR, PlutusV2)?
			.apply_data(pc_params.clone())?;
	let policy_script =
		PlutusScript::from_wrapped_cbor(raw_scripts::VERSION_ORACLE_POLICY, PlutusV2)?
			.apply_data(pc_params.clone())?
			.apply_uplc_data(init_token_asset_data)?
			.apply_uplc_data(validator.plutus_address_data(network)?)?;
	let policy = policy_script.plutus_policy_id();
	let policy_data = PlutusData::BoundedBytes(policy.0.to_vec().into());
	Ok((validator, policy, policy_data))
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

#[cfg(test)]
mod tests {
	use crate::scripts_data::{Addresses, PolicyIds, ScriptsData, ValidatorHashes};
	use cardano_serialization_lib::NetworkIdKind;
	use chain_params::SidechainParams;
	use hex_literal::hex;
	use pretty_assertions::assert_eq;
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
