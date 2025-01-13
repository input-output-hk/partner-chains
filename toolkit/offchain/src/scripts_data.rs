use crate::{csl::NetworkTypeExt, plutus_script::PlutusScript, OffchainError};
use cardano_serialization_lib::{Language, NetworkIdKind};
use ogmios_client::query_network::QueryNetwork;
use serde::Serialize;
use sidechain_domain::{MainchainAddressHash, PolicyId, UtxoId};
use uplc::PlutusData;

/// Provides convenient access to the addresses and hashes of the partner chain smart contracts.
/// Data in this struct is derived from the smart contracts, applied parameters and the network.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptsData {
	pub addresses: Addresses,
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

/// Policy IDs of applied scripts in partner-chains smart contracts.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct PolicyIds {
	pub d_parameter: PolicyId,
	pub permissioned_candidates: PolicyId,
	pub reserve_auth: PolicyId,
	pub version_oracle: PolicyId,
}

pub trait GetScriptsData {
	#[allow(async_fn_in_trait)]
	/// For the given `pc_params` it returns the addresses, hashes and policy ids of the partner chain smart contracts.
	async fn get_scripts_data(&self, genesis_utxo: UtxoId) -> Result<ScriptsData, OffchainError>;
}

impl<T: QueryNetwork> GetScriptsData for T {
	async fn get_scripts_data(&self, genesis_utxo: UtxoId) -> Result<ScriptsData, OffchainError> {
		let network = self
			.shelley_genesis_configuration()
			.await
			.map_err(|e| OffchainError::OgmiosError(e.to_string()))?
			.network
			.to_csl();
		get_scripts_data(genesis_utxo, network)
			.map_err(|e| OffchainError::InternalError(e.to_string()))
	}
}

/// For the given `pc_params` it returns the addresses, hashes and policy ids of the partner chain smart contracts.
pub fn get_scripts_data(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> anyhow::Result<ScriptsData> {
	let version_oracle_data = version_oracle(genesis_utxo, network)?;

	let committee_candidate_validator = PlutusScript::from_wrapped_cbor(
		raw_scripts::COMMITTEE_CANDIDATE_VALIDATOR,
		Language::new_plutus_v2(),
	)?
	.apply_data(genesis_utxo)?;
	let (d_parameter_validator, d_parameter_policy) = d_parameter_scripts(genesis_utxo, network)?;
	let illiquid_circulation_supply_validator = PlutusScript::from_wrapped_cbor(
		raw_scripts::ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR,
		Language::new_plutus_v2(),
	)?
	.apply_uplc_data(version_oracle_data.policy_id_as_plutus_data())?;
	let (permissioned_candidates_validator, permissioned_candidates_policy) =
		permissioned_candidates_scripts(genesis_utxo, network)?;
	let reserve = reserve_scripts(genesis_utxo, network)?;

	Ok(ScriptsData {
		addresses: Addresses {
			committee_candidate_validator: committee_candidate_validator.address_bech32(network)?,
			d_parameter_validator: d_parameter_validator.address_bech32(network)?,
			illiquid_circulation_supply_validator: illiquid_circulation_supply_validator
				.address_bech32(network)?,
			permissioned_candidates_validator: permissioned_candidates_validator
				.address_bech32(network)?,
			reserve_validator: reserve.validator.address_bech32(network)?,
			version_oracle_validator: version_oracle_data.validator.address_bech32(network)?,
		},
		policy_ids: PolicyIds {
			d_parameter: d_parameter_policy.policy_id(),
			permissioned_candidates: permissioned_candidates_policy.policy_id(),
			reserve_auth: reserve.auth_policy.policy_id(),
			version_oracle: version_oracle_data.policy_id(),
		},
	})
}

pub async fn get_scripts_data_with_ogmios(
	genesis_utxo: UtxoId,
	client: impl QueryNetwork,
) -> anyhow::Result<ScriptsData> {
	let network = client.shelley_genesis_configuration().await?.network.to_csl();
	get_scripts_data(genesis_utxo, network)
}

pub(crate) struct VersionOracleData {
	pub(crate) validator: PlutusScript,
	pub(crate) policy: PlutusScript,
}

impl VersionOracleData {
	pub(crate) fn policy_id(&self) -> PolicyId {
		self.policy.policy_id()
	}

	pub(crate) fn policy_id_as_plutus_data(&self) -> PlutusData {
		PlutusData::BoundedBytes(self.policy.script_hash().to_vec().into())
	}
}

// Returns version oracle data required by other scripts.
pub(crate) fn version_oracle(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<VersionOracleData, anyhow::Error> {
	let validator = PlutusScript::from_wrapped_cbor(
		raw_scripts::VERSION_ORACLE_VALIDATOR,
		Language::new_plutus_v2(),
	)?
	.apply_data(genesis_utxo)?;
	let policy_script = PlutusScript::from_wrapped_cbor(
		raw_scripts::VERSION_ORACLE_POLICY,
		Language::new_plutus_v2(),
	)?
	.apply_data(genesis_utxo)?
	.apply_uplc_data(validator.address_data(network)?)?;
	Ok(VersionOracleData { validator, policy: policy_script })
}

pub(crate) fn version_scripts_and_address(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<(PlutusScript, PlutusScript, String), anyhow::Error> {
	let validator = PlutusScript::from_wrapped_cbor(
		raw_scripts::VERSION_ORACLE_VALIDATOR,
		Language::new_plutus_v2(),
	)?
	.apply_data(genesis_utxo)?;
	let policy = PlutusScript::from_wrapped_cbor(
		raw_scripts::VERSION_ORACLE_POLICY,
		Language::new_plutus_v2(),
	)?
	.apply_data(genesis_utxo)?
	.apply_uplc_data(validator.address_data(network)?)?;
	let address = validator.address_bech32(network)?;
	Ok((validator, policy, address))
}

pub(crate) fn d_parameter_scripts(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<(PlutusScript, PlutusScript), anyhow::Error> {
	let version_oracle_data = version_oracle(genesis_utxo, network)?;
	let d_parameter_validator = PlutusScript::from_wrapped_cbor(
		raw_scripts::D_PARAMETER_VALIDATOR,
		Language::new_plutus_v2(),
	)?
	.apply_data(genesis_utxo)?
	.apply_data(version_oracle_data.policy_id())?;
	let d_parameter_policy = PlutusScript::from_wrapped_cbor(
		raw_scripts::D_PARAMETER_POLICY,
		Language::new_plutus_v2(),
	)?
	.apply_data(genesis_utxo)?
	.apply_data(version_oracle_data.policy_id())?
	.apply_uplc_data(d_parameter_validator.address_data(network)?)?;
	Ok((d_parameter_validator, d_parameter_policy))
}

pub(crate) fn permissioned_candidates_scripts(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<(PlutusScript, PlutusScript), anyhow::Error> {
	let version_oracle_data = version_oracle(genesis_utxo, network)?;
	let validator = PlutusScript::from_wrapped_cbor(
		raw_scripts::PERMISSIONED_CANDIDATES_VALIDATOR,
		Language::new_plutus_v2(),
	)?
	.apply_data(genesis_utxo)?
	.apply_data(version_oracle_data.policy_id())?;
	let policy = PlutusScript::from_wrapped_cbor(
		raw_scripts::PERMISSIONED_CANDIDATES_POLICY,
		Language::new_plutus_v2(),
	)?
	.apply_data(genesis_utxo)?
	.apply_data(version_oracle_data.policy_id())?
	.apply_uplc_data(validator.address_data(network)?)?;
	Ok((validator, policy))
}

pub(crate) fn registered_candidates_scripts(
	genesis_utxo: UtxoId,
) -> Result<PlutusScript, anyhow::Error> {
	let validator = PlutusScript::from_wrapped_cbor(
		raw_scripts::COMMITTEE_CANDIDATE_VALIDATOR,
		Language::new_plutus_v2(),
	)?
	.apply_data(genesis_utxo)?;
	Ok(validator)
}

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
		PlutusScript::from_wrapped_cbor(raw_scripts::RESERVE_VALIDATOR, Language::new_plutus_v2())?
			.apply_uplc_data(version_oracle_data.policy_id_as_plutus_data())?;
	let auth_policy = PlutusScript::from_wrapped_cbor(
		raw_scripts::RESERVE_AUTH_POLICY,
		Language::new_plutus_v2(),
	)?
	.apply_uplc_data(version_oracle_data.policy_id_as_plutus_data())?;
	let illiquid_circulation_supply_validator = PlutusScript::from_wrapped_cbor(
		raw_scripts::ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR,
		Language::new_plutus_v2(),
	)?
	.apply_uplc_data(version_oracle_data.policy_id_as_plutus_data())?;
	Ok(ReserveScripts { validator, auth_policy, illiquid_circulation_supply_validator })
}

// Returns the simplest MultiSig policy configuration plutus data:
// there is one required authority and it is the governance authority from sidechain params.
pub(crate) fn multisig_governance_policy_configuration(
	governance_authority: MainchainAddressHash,
) -> PlutusData {
	PlutusData::Array(vec![
		PlutusData::Array(vec![uplc::PlutusData::BoundedBytes(
			governance_authority.0.to_vec().into(),
		)]),
		PlutusData::BigInt(uplc::BigInt::Int(1.into())),
	])
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
