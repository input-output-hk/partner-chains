use crate::plutus_script;
use crate::{csl::NetworkTypeExt, plutus_script::PlutusScript};
use cardano_serialization_lib::NetworkIdKind;
use ogmios_client::query_network::QueryNetwork;
use raw_scripts::ScriptId;
use raw_scripts::{
	COMMITTEE_CANDIDATE_VALIDATOR, D_PARAMETER_POLICY, D_PARAMETER_VALIDATOR, GOVERNED_MAP_POLICY,
	GOVERNED_MAP_VALIDATOR, ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR, PERMISSIONED_CANDIDATES_POLICY,
	PERMISSIONED_CANDIDATES_VALIDATOR, RESERVE_AUTH_POLICY, RESERVE_VALIDATOR,
	VERSION_ORACLE_POLICY, VERSION_ORACLE_VALIDATOR,
};
use serde::Serialize;
use sidechain_domain::{PolicyId, UtxoId};
use uplc::PlutusData;

/// Provides convenient access to the addresses and hashes of the partner chain smart contracts.
/// Data in this struct is derived from the smart contracts, applied parameters and the network.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptsData {
	/// Validator scripts addresses.
	pub addresses: Addresses,
	/// Policy ids.
	pub policy_ids: PolicyIds,
}

/// Bech32 addresses of applied validators in partner-chains smart contracts.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Addresses {
	/// Address of committee candidate validator script
	pub committee_candidate_validator: String,
	/// Address of D-parameter validator script
	pub d_parameter_validator: String,
	/// Address of illiquid circulation supply validator script
	pub illiquid_circulation_supply_validator: String,
	/// Address of permissioned candidates validator script
	pub permissioned_candidates_validator: String,
	/// Address of reserve validator script
	pub reserve_validator: String,
	/// Address of version oracle validator script
	pub version_oracle_validator: String,
	/// Address of governed map validator script
	pub governed_map_validator: String,
}

/// Policy IDs of applied scripts in partner-chains smart contracts.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct PolicyIds {
	/// PolicyId of D-parameter minting policy
	pub d_parameter: PolicyId,
	/// PolicyId of permissioned candidates minting policy
	pub permissioned_candidates: PolicyId,
	/// PolicyId of reserve auth minting policy
	pub reserve_auth: PolicyId,
	/// PolicyId of version oracle minting policy
	pub version_oracle: PolicyId,
	/// PolicyId of governed map minting policy
	pub governed_map: PolicyId,
}

/// Returns [ScriptsData] of the smart contracts for the partner chain identified by `genesis_utxo`.
pub fn get_scripts_data(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> anyhow::Result<ScriptsData> {
	let version_oracle_data = version_oracle(genesis_utxo, network)?;
	let committee_candidate_validator =
		plutus_script![COMMITTEE_CANDIDATE_VALIDATOR, genesis_utxo]?;
	let d_parameter_data = d_parameter_scripts(genesis_utxo, network)?;
	let illiquid_circulation_supply_validator = plutus_script![
		ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR,
		version_oracle_data.policy_id_as_plutus_data()
	]?;
	let permissioned_candidates_data = permissioned_candidates_scripts(genesis_utxo, network)?;
	let reserve = reserve_scripts(genesis_utxo, network)?;
	let governed_map_data = governed_map_scripts(genesis_utxo, network)?;
	Ok(ScriptsData {
		addresses: Addresses {
			committee_candidate_validator: committee_candidate_validator.address_bech32(network)?,
			d_parameter_validator: d_parameter_data.validator_address.clone(),
			illiquid_circulation_supply_validator: illiquid_circulation_supply_validator
				.address_bech32(network)?,
			permissioned_candidates_validator: permissioned_candidates_data
				.validator_address
				.clone(),
			reserve_validator: reserve.validator.address_bech32(network)?,
			version_oracle_validator: version_oracle_data.validator_address.clone(),
			governed_map_validator: governed_map_data.validator_address.clone(),
		},
		policy_ids: PolicyIds {
			d_parameter: d_parameter_data.policy_id(),
			permissioned_candidates: permissioned_candidates_data.policy_id(),
			reserve_auth: reserve.auth_policy.policy_id(),
			version_oracle: version_oracle_data.policy_id(),
			governed_map: governed_map_data.policy_id(),
		},
	})
}

/// Returns [ScriptsData] of the smart contracts for the partner chain identified by `genesis_utxo`,
/// for the network configured in `client`.
pub async fn get_scripts_data_with_ogmios(
	genesis_utxo: UtxoId,
	client: &impl QueryNetwork,
) -> anyhow::Result<ScriptsData> {
	let network = client.shelley_genesis_configuration().await?.network.to_csl();
	get_scripts_data(genesis_utxo, network)
}

/// Type representing a PlutusScript validator and policy pair.
pub struct PlutusScriptData {
	/// Cardano validator script.
	pub validator: PlutusScript,
	/// Bech32 address of `validator`.
	pub validator_address: String,
	/// Cardano minding policy.
	pub policy: PlutusScript,
}

impl PlutusScriptData {
	/// Returns [PolicyId] of [PlutusScriptData].
	pub fn policy_id(&self) -> PolicyId {
		self.policy.policy_id()
	}

	/// Returns [PolicyId] of [PlutusScriptData] as [PlutusData].
	pub fn policy_id_as_plutus_data(&self) -> PlutusData {
		PlutusData::BoundedBytes(self.policy.script_hash().to_vec().into())
	}
}

/// Returns version oracle data required by other scripts.
pub fn version_oracle(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<PlutusScriptData, anyhow::Error> {
	let validator = plutus_script![VERSION_ORACLE_VALIDATOR, genesis_utxo]?;
	let validator_address = validator.address_bech32(network)?;
	let policy =
		plutus_script![VERSION_ORACLE_POLICY, genesis_utxo, validator.address_data(network)?]?;
	Ok(PlutusScriptData { validator, validator_address, policy })
}

pub(crate) fn governed_map_scripts(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<PlutusScriptData, anyhow::Error> {
	let version_oracle_data = version_oracle(genesis_utxo, network)?;
	let validator = plutus_script![
		GOVERNED_MAP_VALIDATOR,
		ScriptId::GovernedMapValidator,
		genesis_utxo,
		version_oracle_data.policy_id()
	]?;
	let validator_address = validator.address_bech32(network)?;
	let policy = plutus_script![
		GOVERNED_MAP_POLICY,
		ScriptId::GovernedMapPolicy,
		genesis_utxo,
		version_oracle_data.policy_id()
	]?;
	Ok(PlutusScriptData { validator, validator_address, policy })
}

pub(crate) fn d_parameter_scripts(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<PlutusScriptData, anyhow::Error> {
	let version_oracle_data = version_oracle(genesis_utxo, network)?;
	let validator =
		plutus_script![D_PARAMETER_VALIDATOR, genesis_utxo, version_oracle_data.policy_id()]?;
	let validator_address = validator.address_bech32(network)?;
	let policy = plutus_script![
		D_PARAMETER_POLICY,
		genesis_utxo,
		version_oracle_data.policy_id(),
		validator.address_data(network)?
	]?;
	Ok(PlutusScriptData { validator, validator_address, policy })
}

pub(crate) fn permissioned_candidates_scripts(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<PlutusScriptData, anyhow::Error> {
	let version_oracle_data = version_oracle(genesis_utxo, network)?;
	let validator = plutus_script![
		PERMISSIONED_CANDIDATES_VALIDATOR,
		genesis_utxo,
		version_oracle_data.policy_id()
	]?;
	let validator_address = validator.address_bech32(network)?;
	let policy = plutus_script![
		PERMISSIONED_CANDIDATES_POLICY,
		genesis_utxo,
		version_oracle_data.policy_id(),
		validator.address_data(network)?
	]?;
	Ok(PlutusScriptData { validator, validator_address, policy })
}

pub(crate) fn registered_candidates_scripts(
	genesis_utxo: UtxoId,
) -> Result<PlutusScript, anyhow::Error> {
	let validator = plutus_script![COMMITTEE_CANDIDATE_VALIDATOR, genesis_utxo]?;
	Ok(validator)
}

#[derive(Clone, Debug)]
pub(crate) struct ReserveScripts {
	pub(crate) validator: PlutusScript,
	pub(crate) auth_policy: PlutusScript,
	pub(crate) illiquid_circulation_supply_validator: PlutusScript,
}

pub(crate) fn reserve_scripts(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<ReserveScripts, anyhow::Error> {
	let version_oracle_data = version_oracle(genesis_utxo, network)?;
	let validator =
		plutus_script![RESERVE_VALIDATOR, version_oracle_data.policy_id_as_plutus_data()]?;
	let auth_policy =
		plutus_script![RESERVE_AUTH_POLICY, version_oracle_data.policy_id_as_plutus_data()]?;
	let illiquid_circulation_supply_validator = plutus_script![
		ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR,
		version_oracle_data.policy_id_as_plutus_data()
	]?;
	Ok(ReserveScripts { validator, auth_policy, illiquid_circulation_supply_validator })
}

#[cfg(test)]
mod tests {
	use crate::scripts_data::{Addresses, PolicyIds, ScriptsData};
	use cardano_serialization_lib::NetworkIdKind;
	use hex_literal::hex;
	use pretty_assertions::assert_eq;
	use sidechain_domain::{McTxHash, PolicyId, UtxoId};

	pub(crate) const TEST_PARAMS: UtxoId = UtxoId {
		tx_hash: McTxHash(hex!("8ea10040249ad3033ae7c4d4b69e0b2e2b50a90741b783491cb5ddf8ced0d861")),
		index: sidechain_domain::UtxoIndex(0),
	};

	pub(crate) fn scripts_data_test_vector() -> ScriptsData {
		ScriptsData {
			addresses: Addresses {
				committee_candidate_validator:
					"addr_test1wz8z7eaac04rp75u47vqy9k3qgwcxxh92ff3h3hp2xae4kgzwdwp3".into(),
				d_parameter_validator:
					"addr_test1wppqfuvptxq3rwv2qwke2dkh8vd0mmc8k4r6a44k86tpckss2zg4v".into(),
				permissioned_candidates_validator:
					"addr_test1wql3vzrgx0hdqhxt4n2wvv93mhnglphl0fadkd06zuzkglcnyc09n".into(),
				illiquid_circulation_supply_validator:
					"addr_test1wq7crkpl5mpdeq9wyqyvr2u7q7gtv02pjxgl3fh8mv5r6ec00zynf".into(),
				reserve_validator:
					"addr_test1wqs5y7fn6sns7v7eey94mj2wd7ysadr3zmstjfzhk0frdtgsm8pgk".into(),
				version_oracle_validator:
					"addr_test1wqxm9e576k5ew7g7ctuqx77p9u7zytesnjsx54q2etck00gqplk0l".into(),
				governed_map_validator:
					"addr_test1wpyjqa4deje4c9jsjzt3djv28zszvatzup60vvjvn5cdqeggfmj6v".into(),
			},
			policy_ids: PolicyIds {
				d_parameter: PolicyId(hex!(
					"f30c3f90c342e61b3f34042bcabd7be8f3ec4b7a6857fdfcdb7b7936"
				)),
				permissioned_candidates: PolicyId(hex!(
					"8dbf5934f4870570752205fbda4796b5f7df30c9b5a009452dc7df5f"
				)),
				reserve_auth: PolicyId(hex!(
					"8c2f422162225752e5842fd7ac103d6e679e21c13514c9ef8224452c"
				)),
				version_oracle: PolicyId(hex!(
					"aa7f601aa9f441a26823d872f052d52767229f3301567c86475dfcfb"
				)),
				governed_map: PolicyId(hex!(
					"951a84ca81e42b464934b100de79a3c1e4cbaa81ec68602b625add54"
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
