//! Functionality related to filtering invalid candidates from the candidates

use frame_support::pallet_prelude::TypeInfo;
use parity_scale_codec::{Decode, Encode};
use plutus::*;
use plutus_datum_derive::ToDatum;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sidechain_domain::*;
use sp_core::{ecdsa, ed25519, sr25519};
use sp_runtime::traits::Verify;

/// Signed Message of the Authority Candidate to register
/// It's ToDatum implementation has to produce datum that has to match main chain structure,
/// because it will be hashed and signed for signature verification.
#[derive(Debug, ToDatum)]
pub struct RegisterValidatorSignedMessage<Params> {
	pub sidechain_params: Params,
	pub sidechain_pub_key: Vec<u8>,
	/// UTxO that is an input parameter to the registration transaction
	pub input_utxo: UtxoId,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct CandidateWithStake<TAccountId, TAccountKeys> {
	pub candidate: Candidate<TAccountId, TAccountKeys>,
	/// Amount of ADA staked/locked by the Authority Candidate
	pub stake_delegation: StakeDelegation,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq)]
pub struct Candidate<TAccountId, TAccountKeys> {
	pub account_id: TAccountId,
	pub account_keys: TAccountKeys,
}

/// Get the valid trustless candidates from the registrations from inherent data
pub fn filter_trustless_candidates_registrations<
	TAccountId,
	TAccountKeys,
	Params: ToDatum + Clone,
>(
	candidate_registrations: Vec<CandidateRegistrations>,
	sidechain_params: Params,
) -> Vec<CandidateWithStake<TAccountId, TAccountKeys>>
where
	TAccountKeys: From<(sr25519::Public, ed25519::Public)>,
	TAccountId: From<ecdsa::Public>,
{
	candidate_registrations
		.into_iter()
		.flat_map(|candidate_registrations| {
			select_latest_valid_candidate(candidate_registrations, &sidechain_params)
		})
		.collect()
}

pub fn filter_invalid_permissioned_candidates<TAccountId, TAccountKeys>(
	permissioned_candidates: Vec<PermissionedCandidateData>,
) -> Vec<Candidate<TAccountId, TAccountKeys>>
where
	TAccountKeys: From<(sr25519::Public, ed25519::Public)>,
	TAccountId: TryFrom<sidechain_domain::SidechainPublicKey>,
{
	permissioned_candidates
		.into_iter()
		.filter_map(|candidate| {
			let (account_id, aura_key, grandpa_key) =
				validate_permissioned_candidate_data(candidate).ok()?;
			let account_keys = (aura_key, grandpa_key).into();
			Some(Candidate { account_id, account_keys })
		})
		.collect()
}

fn select_latest_valid_candidate<TAccountId, TAccountKeys, Params: ToDatum + Clone>(
	candidate_registrations: CandidateRegistrations,
	sidechain_params: &Params,
) -> Option<CandidateWithStake<TAccountId, TAccountKeys>>
where
	TAccountId: From<ecdsa::Public>,
	TAccountKeys: From<(sr25519::Public, ed25519::Public)>,
{
	let stake_delegation = validate_stake(candidate_registrations.stake_delegation).ok()?;
	let mainchain_pub_key = candidate_registrations.mainchain_pub_key;

	let (candidate_data, _) = candidate_registrations
		.registrations
		.into_iter()
		.filter_map(|registration_data| {
			match validate_registration_data(
				&mainchain_pub_key,
				&registration_data,
				sidechain_params,
			) {
				Ok(candidate) => Some((candidate, registration_data.utxo_info)),
				Err(_) => None,
			}
		})
		// Get the latest valid registration of the authority candidate
		.max_by_key(|(_, utxo_info)| utxo_info.ordering_key())?;

	Some(CandidateWithStake {
		candidate: Candidate {
			account_id: candidate_data.account_id.into(),
			account_keys: candidate_data.account_keys.into(),
		},
		stake_delegation,
	})
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(thiserror::Error, Serialize, Deserialize))]
pub enum StakeError {
	#[cfg_attr(feature = "std", error("Stake should be greater than 0"))]
	InvalidStake,
	#[cfg_attr(feature = "std", error("Stake delegation information cannot be computed yet. Registration will turn valid if stake delegation for the epoch will be greater than 0"))]
	UnknownStake,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(thiserror::Error, Serialize, Deserialize))]
pub enum RegistrationDataError {
	#[cfg_attr(feature = "std", error("Registration data is invalid: InvalidMainchainSignature"))]
	InvalidMainchainSignature,
	#[cfg_attr(feature = "std", error("Registration data is invalid: InvalidSidechainSignature"))]
	InvalidSidechainSignature,
	#[cfg_attr(feature = "std", error("Registration data is invalid: InvalidTxInput"))]
	InvalidTxInput,
	#[cfg_attr(feature = "std", error("Registration data is invalid: InvalidMainchainPubKey"))]
	InvalidMainchainPubKey,
	#[cfg_attr(feature = "std", error("Registration data is invalid: InvalidSidechainPubKey"))]
	InvalidSidechainPubKey,
	#[cfg_attr(feature = "std", error("Registration data is invalid: InvalidAuraKey"))]
	InvalidAuraKey,
	#[cfg_attr(feature = "std", error("Registration data is invalid: InvalidGrandpaKey"))]
	InvalidGrandpaKey,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(thiserror::Error, Serialize, Deserialize))]
pub enum PermissionedCandidateDataError {
	#[cfg_attr(
		feature = "std",
		error("Permissioned candidate data is invalid: InvalidSidechainPubKey")
	)]
	InvalidSidechainPubKey,
	#[cfg_attr(feature = "std", error("Permissioned candidate data is invalid: InvalidAuraKey"))]
	InvalidAuraKey,
	#[cfg_attr(
		feature = "std",
		error("Permissioned candidate data is invalid: InvalidGrandpaKey")
	)]
	InvalidGrandpaKey,
}

pub fn validate_permissioned_candidate_data<AccountId: TryFrom<SidechainPublicKey>>(
	candidate: PermissionedCandidateData,
) -> Result<(AccountId, sr25519::Public, ed25519::Public), PermissionedCandidateDataError> {
	Ok((
		candidate
			.sidechain_public_key
			.try_into()
			.map_err(|_| PermissionedCandidateDataError::InvalidSidechainPubKey)?,
		candidate
			.aura_public_key
			.try_into_sr25519()
			.ok_or(PermissionedCandidateDataError::InvalidAuraKey)?,
		candidate
			.grandpa_public_key
			.try_into_ed25519()
			.ok_or(PermissionedCandidateDataError::InvalidGrandpaKey)?,
	))
}

/// Is the registration data provided by the authority candidate valid?
pub fn validate_registration_data<Params: ToDatum + Clone>(
	mainchain_pub_key: &MainchainPublicKey,
	registration_data: &RegistrationData,
	sidechain_params: &Params,
) -> Result<Candidate<ecdsa::Public, (sr25519::Public, ed25519::Public)>, RegistrationDataError> {
	let aura_pub_key = registration_data
		.aura_pub_key
		.try_into_sr25519()
		.ok_or(RegistrationDataError::InvalidAuraKey)?;
	let grandpa_pub_key = registration_data
		.grandpa_pub_key
		.try_into_ed25519()
		.ok_or(RegistrationDataError::InvalidGrandpaKey)?;
	let sidechain_pub_key = ecdsa::Public::from(
		<[u8; 33]>::try_from(registration_data.sidechain_pub_key.0.clone())
			.map_err(|_| RegistrationDataError::InvalidSidechainPubKey)?,
	);

	let signed_message = RegisterValidatorSignedMessage {
		sidechain_params: sidechain_params.clone(),
		sidechain_pub_key: registration_data.sidechain_pub_key.0.clone(),
		input_utxo: registration_data.consumed_input,
	};

	let signed_message_encoded = minicbor::to_vec(signed_message.to_datum())
		.expect("`RegisterValidatorSignedMessage` should always be encodable");

	verify_mainchain_signature(mainchain_pub_key, registration_data, &signed_message_encoded)?;
	verify_sidechain_signature(
		sidechain_pub_key,
		&registration_data.sidechain_signature,
		&signed_message_encoded,
	)?;
	verify_tx_inputs(registration_data)?;

	// TODO - Stake Validation: https://input-output.atlassian.net/browse/ETCM-4082

	Ok(Candidate { account_id: sidechain_pub_key, account_keys: (aura_pub_key, grandpa_pub_key) })
}

pub fn validate_stake(stake: Option<StakeDelegation>) -> Result<StakeDelegation, StakeError> {
	match stake {
		None => Err(StakeError::UnknownStake),
		Some(stake) => {
			if stake.is_zero() {
				Err(StakeError::InvalidStake)
			} else {
				Ok(stake)
			}
		},
	}
}

fn verify_mainchain_signature(
	mainchain_pub_key: &MainchainPublicKey,
	registration_data: &RegistrationData,
	signed_message_encoded: &[u8],
) -> Result<(), RegistrationDataError> {
	let mainchain_signature: [u8; 64] = registration_data
		.mainchain_signature
		.0
		.clone()
		.try_into()
		.map_err(|_| RegistrationDataError::InvalidMainchainSignature)?;
	let mainchain_signature = ed25519::Signature::from(mainchain_signature);
	if mainchain_signature
		.verify(signed_message_encoded, &ed25519::Public::from(mainchain_pub_key.0))
	{
		Ok(())
	} else {
		Err(RegistrationDataError::InvalidMainchainSignature)
	}
}

// Pre EIP155 signature.v convention
const NEGATIVE_POINT_SIGN: u8 = 27;
const POSITIVE_POINT_SIGN: u8 = 28;

fn verify_sidechain_signature(
	pub_key: ecdsa::Public,
	signature: &SidechainSignature,
	signed_message_encoded: &[u8],
) -> Result<(), RegistrationDataError> {
	let sidechain_signature = <[u8; 64]>::try_from(signature.0.clone())
		.map_err(|_| RegistrationDataError::InvalidSidechainSignature)?;
	let mut sidechain_signature_with_v = [0u8; 65];
	sidechain_signature_with_v[..64].copy_from_slice(&sidechain_signature);
	// TODO: Extract to crypto util. See https://github.com/input-output-hk/partner-chains/pull/61#discussion_r1293205895
	let is_valid = [NEGATIVE_POINT_SIGN, POSITIVE_POINT_SIGN].iter().any(|v| {
		sidechain_signature_with_v[64] = *v;
		ecdsa::Signature::from(sidechain_signature_with_v).verify(signed_message_encoded, &pub_key)
	});
	if is_valid {
		Ok(())
	} else {
		Err(RegistrationDataError::InvalidSidechainSignature)
	}
}

fn verify_tx_inputs(registration_data: &RegistrationData) -> Result<(), RegistrationDataError> {
	if registration_data.tx_inputs.contains(&registration_data.consumed_input) {
		Ok(())
	} else {
		Err(RegistrationDataError::InvalidTxInput)
	}
}

// When implementing, make sure that the same validation is used here and in the committee selection logic
sp_api::decl_runtime_apis! {
	pub trait CandidateValidationApi {
		fn validate_registered_candidate_data(mainchain_pub_key: &MainchainPublicKey, registration_data: &RegistrationData) -> Option<RegistrationDataError>;
		fn validate_stake(stake: Option<StakeDelegation>) -> Option<StakeError>;
		fn validate_permissioned_candidate_data(candidate: PermissionedCandidateData) -> Option<PermissionedCandidateDataError>;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tests::{AccountId, AccountKeys};
	use chain_params::SidechainParams;
	use hex_literal::hex;
	use sp_core::Pair;

	/// Get Valid Parameters of the `is_registration_data_valid()` function
	fn create_valid_parameters() -> (MainchainPublicKey, RegistrationData, SidechainParams) {
		let input_utxo = UtxoId {
			tx_hash: McTxHash(hex!(
				"d260a76b267e27fdf79c217ec61b776d6436dc78eefeac8f3c615486a71f38eb"
			)),
			index: UtxoIndex(1),
		};

		let registration_data = RegistrationData {
			consumed_input: input_utxo,
			sidechain_signature: SidechainSignature(
				hex!("f31f26ea682a5721cd07cb337a3a7ca134d3909f6afcd09c74a67dda35f28aa20983e396cb444ba87d146ab3bf9ecf2c129572decfde7db9cfb2580e429d8744").to_vec()
			),
			mainchain_signature: MainchainSignature(
				hex!("1ff8cd26c9132bed8b54acb13d4210cc38fb6577c548222d3a976e1cbf6cdc3dff94922aa3aad6b06a87ce8e15fd254fac14f6654ced49dc8758a6095f347604").to_vec()
			),
			cross_chain_signature: CrossChainSignature(
				hex!("4b3a74688573be4a3b68f0dd8f9ef699a5bae594d8f5f915394afacf259ecada2f5d85dc2a54550ce17efa26c0e2937be188666a9fb25ab6d467d52751144bf1").to_vec()
			),
			sidechain_pub_key: SidechainPublicKey(
				hex!("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").to_vec()
			),
			cross_chain_pub_key: CrossChainPublicKey(
				hex!("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").to_vec()
			),
			aura_pub_key: AuraPublicKey(hex!(
					"d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
				).to_vec()),
			grandpa_pub_key: GrandpaPublicKey(hex!(
					"88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee"
				).to_vec()),
			utxo_info: UtxoInfo {
				utxo_id: UtxoId {
					tx_hash: McTxHash(hex!("a32208949e4fe2989f4c4dc3409850af79f266c8603e1d34a330b4d27e03854a")),
					index: UtxoIndex(0)
				},
				epoch_number: McEpochNumber(248),
				block_number: McBlockNumber(941619),
				slot_number: McSlotNumber(21450270),
				tx_index_within_block: McTxIndexInBlock(2),
			},
			tx_inputs: vec![input_utxo],
		};

		let sidechain_params = SidechainParams {
			chain_id: 0,
			genesis_committee_utxo: UtxoId {
				tx_hash: McTxHash(hex!(
					"f17e6d3aa72095e04489d13d776bf05a66b5a8c49d89397c28b18a1784b9950e"
				)),
				index: UtxoIndex(0),
			},
			threshold_numerator: 2,
			threshold_denominator: 3,
			governance_authority: MainchainAddressHash(hex!(
				"00112233445566778899001122334455667788990011223344556677"
			)),
		};

		(
			MainchainPublicKey(hex!(
				"fb335cabe7d3dd77d0177cd332e9a44998d9d5085b811650853b7bb0752a8bef"
			)),
			registration_data,
			sidechain_params,
		)
	}

	mod is_registration_data_valid_tests {
		use super::*;
		use sidechain_domain::{
			CrossChainPublicKey, McTxHash, SidechainPublicKey, SidechainSignature,
		};
		use sp_core::Pair;

		fn create_parameters(
			signing_sidechain_account: ecdsa::Pair,
			sidechain_pub_key: Vec<u8>,
		) -> (MainchainPublicKey, RegistrationData, SidechainParams) {
			let mainchain_account = ed25519::Pair::from_seed_slice(&[7u8; 32]).unwrap();

			let signed_message = RegisterValidatorSignedMessage {
				sidechain_params: SidechainParams {
					chain_id: 101,
					genesis_committee_utxo: UtxoId {
						tx_hash: McTxHash([7u8; TX_HASH_SIZE]),
						index: UtxoIndex(0),
					},
					threshold_numerator: 2,
					threshold_denominator: 3,
					governance_authority: MainchainAddressHash(hex!(
						"00112233445566778899001122334455667788990011223344556677"
					)),
				},
				sidechain_pub_key: sidechain_pub_key.clone(),
				input_utxo: UtxoId { tx_hash: McTxHash([7u8; TX_HASH_SIZE]), index: UtxoIndex(0) },
			};

			let signed_message_encoded = minicbor::to_vec(signed_message.to_datum()).unwrap();

			let mainchain_signature = mainchain_account.sign(&signed_message_encoded[..]);
			let sidechain_signature =
				signing_sidechain_account.sign(&signed_message_encoded[..]).0.as_slice()[..64]
					.to_vec();

			let registration_data = RegistrationData {
				consumed_input: signed_message.input_utxo,
				sidechain_signature: SidechainSignature(sidechain_signature),
				mainchain_signature: MainchainSignature(mainchain_signature.0.to_vec()),
				cross_chain_signature: CrossChainSignature(vec![]),
				sidechain_pub_key: SidechainPublicKey(sidechain_pub_key),
				cross_chain_pub_key: CrossChainPublicKey(vec![]),
				aura_pub_key: AuraPublicKey(vec![1; 32]),
				grandpa_pub_key: GrandpaPublicKey(vec![2; 32]),
				utxo_info: UtxoInfo {
					utxo_id: UtxoId { tx_hash: McTxHash([7u8; 32]), index: UtxoIndex(7) },
					epoch_number: McEpochNumber(7),
					block_number: McBlockNumber(7),
					slot_number: McSlotNumber(7),
					tx_index_within_block: McTxIndexInBlock(7),
				},
				tx_inputs: vec![signed_message.input_utxo],
			};

			(
				MainchainPublicKey(mainchain_account.public().0),
				registration_data,
				signed_message.sidechain_params,
			)
		}

		#[test]
		fn should_work() {
			let (mainchain_pub_key, registration_data, sidechain_params) =
				create_valid_parameters();
			assert!(validate_registration_data(
				&mainchain_pub_key,
				&registration_data,
				&sidechain_params,
			)
			.is_ok());
		}

		#[test]
		fn should_not_work_if_mainchain_pub_key_is_different() {
			let (mainchain_pub_key, registration_data, sidechain_params) =
				create_valid_parameters();
			let different_mainchain_pub_key =
				MainchainPublicKey(ed25519::Pair::from_seed_slice(&[0u8; 32]).unwrap().public().0);
			assert_ne!(mainchain_pub_key, different_mainchain_pub_key);
			assert_eq!(
				validate_registration_data(
					&different_mainchain_pub_key,
					&registration_data,
					&sidechain_params,
				),
				Err(RegistrationDataError::InvalidMainchainSignature)
			);
		}

		#[test]
		fn should_not_work_if_sidechain_pub_key_is_different() {
			let signing_sidechain_account = ecdsa::Pair::from_seed_slice(&[77u8; 32]).unwrap();
			let sidechain_pub_key =
				ecdsa::Pair::from_seed_slice(&[123u8; 32]).unwrap().public().0.to_vec();
			let (mainchain_pub_key, registration_data, sidechain_params) =
				create_parameters(signing_sidechain_account, sidechain_pub_key);
			assert_eq!(
				validate_registration_data(
					&mainchain_pub_key,
					&registration_data,
					&sidechain_params,
				),
				Err(RegistrationDataError::InvalidSidechainSignature)
			);
		}

		#[test]
		fn should_fail_validation_for_invalid_grandpa_key() {
			let signing_sidechain_account = ecdsa::Pair::from_seed_slice(&[77u8; 32]).unwrap();
			let sidechain_pub_key =
				ecdsa::Pair::from_seed_slice(&[123u8; 32]).unwrap().public().0.to_vec();
			let (mainchain_pub_key, mut registration_data, sidechain_params) =
				create_parameters(signing_sidechain_account, sidechain_pub_key);
			registration_data.grandpa_pub_key = GrandpaPublicKey(vec![3; 4]);
			assert_eq!(
				validate_registration_data(
					&mainchain_pub_key,
					&registration_data,
					&sidechain_params,
				),
				Err(RegistrationDataError::InvalidGrandpaKey)
			);
		}

		#[test]
		fn should_fail_validation_for_invalid_aura_key() {
			let signing_sidechain_account = ecdsa::Pair::from_seed_slice(&[77u8; 32]).unwrap();
			let sidechain_pub_key =
				ecdsa::Pair::from_seed_slice(&[123u8; 32]).unwrap().public().0.to_vec();
			let (mainchain_pub_key, mut registration_data, sidechain_params) =
				create_parameters(signing_sidechain_account, sidechain_pub_key);
			registration_data.aura_pub_key = AuraPublicKey(vec![3; 4]);
			assert_eq!(
				validate_registration_data(
					&mainchain_pub_key,
					&registration_data,
					&sidechain_params,
				),
				Err(RegistrationDataError::InvalidAuraKey)
			);
		}

		#[test]
		fn should_not_work_if_sidechain_params_is_different() {
			let (mainchain_pub_key, registration_data, sidechain_params) =
				create_valid_parameters();
			let different_sidechain_params = SidechainParams {
				chain_id: sidechain_params.chain_id + 1,
				..sidechain_params.clone()
			};
			assert_ne!(different_sidechain_params, sidechain_params);
			assert!(validate_registration_data(
				&mainchain_pub_key,
				&registration_data,
				&different_sidechain_params,
			)
			.is_err());
		}

		#[test]
		fn should_not_work_if_tx_input_is_invalid() {
			let (mainchain_pub_key, mut registration_data, sidechain_params) =
				create_valid_parameters();
			registration_data.tx_inputs = vec![];
			assert_eq!(
				validate_registration_data(
					&mainchain_pub_key,
					&registration_data,
					&sidechain_params,
				),
				Err(RegistrationDataError::InvalidTxInput)
			);
		}
	}

	#[test]
	fn should_filter_out_candidates_with_invalid_stake() {
		let (mc_pub_key, registration_data, sidechain_params) = create_valid_parameters();
		let candidate_registrations = vec![
			CandidateRegistrations {
				mainchain_pub_key: mc_pub_key.clone(),
				registrations: vec![registration_data.clone()],
				stake_delegation: Some(StakeDelegation(0)),
			},
			CandidateRegistrations {
				mainchain_pub_key: mc_pub_key.clone(),
				registrations: vec![registration_data.clone()],
				stake_delegation: Some(StakeDelegation(1)),
			},
			CandidateRegistrations {
				mainchain_pub_key: mc_pub_key.clone(),
				registrations: vec![registration_data.clone()],
				stake_delegation: None,
			},
			CandidateRegistrations {
				mainchain_pub_key: mc_pub_key,
				registrations: vec![registration_data],
				stake_delegation: Some(StakeDelegation(2)),
			},
		];

		let valid_candidates = filter_trustless_candidates_registrations::<AccountId, AccountKeys, _>(
			candidate_registrations,
			sidechain_params,
		);

		assert_eq!(valid_candidates.len(), 2);
		assert_eq!(valid_candidates[0].stake_delegation, StakeDelegation(1));
		assert_eq!(valid_candidates[1].stake_delegation, StakeDelegation(2));
	}

	#[test]
	fn should_filter_out_permissioned_candidates_with_invalid_keys() {
		let valid_sidechain_pub_key = SidechainPublicKey(
			ecdsa::Pair::from_seed_slice(&[123u8; 32]).unwrap().public().0.to_vec(),
		);
		let valid_candidate = PermissionedCandidateData {
			sidechain_public_key: valid_sidechain_pub_key.clone(),
			aura_public_key: AuraPublicKey(
				hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d").into(),
			),
			grandpa_public_key: GrandpaPublicKey(
				hex!("88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee").into(),
			),
		};
		let permissioned_candidates = vec![
			PermissionedCandidateData {
				aura_public_key: AuraPublicKey(vec![1; 2]),
				..valid_candidate.clone()
			},
			PermissionedCandidateData {
				sidechain_public_key: SidechainPublicKey(vec![2; 2]),
				..valid_candidate.clone()
			},
			PermissionedCandidateData {
				grandpa_public_key: GrandpaPublicKey(vec![3; 4]),
				..valid_candidate.clone()
			},
			valid_candidate.clone(),
		];
		let valid_candidates = filter_invalid_permissioned_candidates::<AccountId, AccountKeys>(
			permissioned_candidates,
		);
		assert_eq!(valid_candidates.len(), 1);
		assert_eq!(
			valid_candidates.first().unwrap().account_id,
			valid_sidechain_pub_key.try_into().unwrap()
		);
		assert_eq!(
			valid_candidates.first().unwrap().account_keys,
			(
				valid_candidate.aura_public_key.try_into_sr25519().unwrap(),
				valid_candidate.grandpa_public_key.try_into_ed25519().unwrap()
			)
				.into()
		);
	}
}
