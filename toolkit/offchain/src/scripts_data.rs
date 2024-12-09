use crate::{csl::NetworkTypeExt, plutus_script::PlutusScript, OffchainError};
use cardano_serialization_lib::{LanguageKind::PlutusV2, NetworkIdKind};
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
	pub permissioned_candidates: PolicyId,
	pub reserve_auth: PolicyId,
	pub version_oracle: PolicyId,
	pub governance: PolicyId,
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
	let (version_oracle_validator, version_oracle_policy, version_oracle_policy_data) =
		version_oracle(genesis_utxo, network)?;

	let committee_candidate_validator =
		PlutusScript::from_wrapped_cbor(raw_scripts::COMMITTEE_CANDIDATE_VALIDATOR, PlutusV2)?
			.apply_data(genesis_utxo)?;
	let (d_parameter_validator, d_parameter_policy) = d_parameter_scripts(genesis_utxo, network)?;
	let illiquid_circulation_supply_validator = PlutusScript::from_wrapped_cbor(
		raw_scripts::ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR,
		PlutusV2,
	)?
	.apply_uplc_data(version_oracle_policy_data.clone())?;
	let (permissioned_candidates_validator, permissioned_candidates_policy) =
		permissioned_candidates_scripts(genesis_utxo, network)?;

	let reserve_validator =
		PlutusScript::from_wrapped_cbor(raw_scripts::RESERVE_VALIDATOR, PlutusV2)?
			.apply_uplc_data(version_oracle_policy_data.clone())?;

	let governance_policy =
		PlutusScript::from_wrapped_cbor(raw_scripts::MULTI_SIG_POLICY, PlutusV2)?.apply_uplc_data(
			multisig_governance_policy_configuration(MainchainAddressHash::from_vkey([0u8; 32])),
		)?;
	let reserve_auth_policy =
		PlutusScript::from_wrapped_cbor(raw_scripts::RESERVE_AUTH_POLICY, PlutusV2)?
			.apply_uplc_data(version_oracle_policy_data)?;

	Ok(ScriptsData {
		addresses: Addresses {
			committee_candidate_validator: committee_candidate_validator.address_bech32(network)?,
			d_parameter_validator: d_parameter_validator.address_bech32(network)?,
			illiquid_circulation_supply_validator: illiquid_circulation_supply_validator
				.address_bech32(network)?,
			permissioned_candidates_validator: permissioned_candidates_validator
				.address_bech32(network)?,
			reserve_validator: reserve_validator.address_bech32(network)?,
			version_oracle_validator: version_oracle_validator.address_bech32(network)?,
		},
		validator_hashes: ValidatorHashes {
			committee_candidate_validator: committee_candidate_validator.script_address(),
			d_parameter_validator: d_parameter_validator.script_address(),
			illiquid_circulation_supply_validator: illiquid_circulation_supply_validator
				.script_address(),
			permissioned_candidates_validator: permissioned_candidates_validator.script_address(),
			reserve_validator: reserve_validator.script_address(),
			version_oracle_validator: version_oracle_validator.script_address(),
		},
		policy_ids: PolicyIds {
			d_parameter: d_parameter_policy.policy_id(),
			permissioned_candidates: permissioned_candidates_policy.policy_id(),
			reserve_auth: reserve_auth_policy.policy_id(),
			version_oracle: version_oracle_policy,
			governance: governance_policy.policy_id(),
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

// Returns version oracle script, policy and PlutusData required by other scripts.
pub(crate) fn version_oracle(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<(PlutusScript, PolicyId, PlutusData), anyhow::Error> {
	let validator =
		PlutusScript::from_wrapped_cbor(raw_scripts::VERSION_ORACLE_VALIDATOR, PlutusV2)?
			.apply_data(genesis_utxo)?;
	let policy_script =
		PlutusScript::from_wrapped_cbor(raw_scripts::VERSION_ORACLE_POLICY, PlutusV2)?
			.apply_data(genesis_utxo)?
			.apply_uplc_data(validator.address_data(network)?)?;
	let policy = policy_script.policy_id();
	let policy_data = PlutusData::BoundedBytes(policy.0.to_vec().into());
	Ok((validator, policy, policy_data))
}

pub(crate) fn version_scripts_and_address(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<(PlutusScript, PlutusScript, String), anyhow::Error> {
	let validator =
		PlutusScript::from_wrapped_cbor(raw_scripts::VERSION_ORACLE_VALIDATOR, PlutusV2)?
			.apply_data(genesis_utxo)?;
	let policy = PlutusScript::from_wrapped_cbor(raw_scripts::VERSION_ORACLE_POLICY, PlutusV2)?
		.apply_data(genesis_utxo)?
		.apply_uplc_data(validator.address_data(network)?)?;
	let address = validator.address_bech32(network)?;
	Ok((validator, policy, address))
}

pub(crate) fn d_parameter_scripts(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<(PlutusScript, PlutusScript), anyhow::Error> {
	let (_, version_oracle_policy, _) = version_oracle(genesis_utxo, network)?;
	let d_parameter_validator =
		PlutusScript::from_wrapped_cbor(raw_scripts::D_PARAMETER_VALIDATOR, PlutusV2)?
			.apply_data(genesis_utxo)?
			.apply_data(version_oracle_policy.clone())?;
	let d_parameter_policy =
		PlutusScript::from_wrapped_cbor(raw_scripts::D_PARAMETER_POLICY, PlutusV2)?
			.apply_data(genesis_utxo)?
			.apply_data(version_oracle_policy.clone())?
			.apply_uplc_data(d_parameter_validator.address_data(network)?)?;
	Ok((d_parameter_validator, d_parameter_policy))
}

pub(crate) fn permissioned_candidates_scripts(
	genesis_utxo: UtxoId,
	network: NetworkIdKind,
) -> Result<(PlutusScript, PlutusScript), anyhow::Error> {
	let (_, version_oracle_policy, _) = version_oracle(genesis_utxo, network)?;
	let validator =
		PlutusScript::from_wrapped_cbor(raw_scripts::PERMISSIONED_CANDIDATES_VALIDATOR, PlutusV2)?
			.apply_data(genesis_utxo)?
			.apply_data(version_oracle_policy.clone())?;
	let policy =
		PlutusScript::from_wrapped_cbor(raw_scripts::PERMISSIONED_CANDIDATES_POLICY, PlutusV2)?
			.apply_data(genesis_utxo)?
			.apply_data(version_oracle_policy.clone())?
			.apply_uplc_data(validator.address_data(network)?)?;
	Ok((validator, policy))
}

pub(crate) fn registered_candidates_scripts(
	genesis_utxo: UtxoId,
) -> Result<PlutusScript, anyhow::Error> {
	let validator =
		PlutusScript::from_wrapped_cbor(raw_scripts::COMMITTEE_CANDIDATE_VALIDATOR, PlutusV2)?
			.apply_data(genesis_utxo)?;
	Ok(validator)
}

// Returns the simplest MultiSig policy configuration plutus data:
// there is one required authority and it is the governance authority from sidechain params.
fn multisig_governance_policy_configuration(
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
	use crate::scripts_data::{Addresses, PolicyIds, ScriptsData, ValidatorHashes};
	use cardano_serialization_lib::NetworkIdKind;
	use hex_literal::hex;
	use pretty_assertions::assert_eq;
	use sidechain_domain::{MainchainAddressHash, McTxHash, PolicyId, UtxoId};

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
			validator_hashes: ValidatorHashes {
				committee_candidate_validator: MainchainAddressHash(hex!(
					"8e2f67bdc3ea30fa9caf980216d1021d831ae552531bc6e151bb9ad9"
				)),
				d_parameter_validator: MainchainAddressHash(hex!(
					"4204f181598111b98a03ad9536d73b1afdef07b547aed6b63e961c5a"
				)),
				illiquid_circulation_supply_validator: MainchainAddressHash(hex!(
					"3d81d83fa6c2dc80ae2008c1ab9e0790b63d419191f8a6e7db283d67"
				)),
				permissioned_candidates_validator: MainchainAddressHash(hex!(
					"3f16086833eed05ccbacd4e630b1dde68f86ff7a7adb35fa1705647f"
				)),
				reserve_validator: MainchainAddressHash(hex!(
					"21427933d4270f33d9c90b5dc94e6f890eb47116e0b92457b3d236ad"
				)),
				version_oracle_validator: MainchainAddressHash(hex!(
					"0db2e69ed5a997791ec2f8037bc12f3c222f309ca06a540acaf167bd"
				)),
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
				// TODO get correct hash
				governance: PolicyId(hex!(
					"9a2738df5cd08458700444b278293f9ba9325c0029ae6d5d36e8678a"
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
