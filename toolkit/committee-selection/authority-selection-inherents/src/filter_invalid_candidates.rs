//! Functionality related to filtering invalid candidates from the candidates

use crate::CommitteeMember;
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
pub struct RegisterValidatorSignedMessage {
	pub genesis_utxo: UtxoId,
	pub sidechain_pub_key: Vec<u8>,
	/// UTxO that is an input parameter to the registration transaction
	pub registration_utxo: UtxoId,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord)]
pub struct CandidateWithStake<TAccountId, TAccountKeys> {
	pub stake_pool_pub_key: StakePoolPublicKey,
	/// Amount of ADA staked/locked by the Authority Candidate
	pub stake_delegation: StakeDelegation,
	pub account_id: TAccountId,
	pub account_keys: TAccountKeys,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord)]
pub struct PermissionedCandidate<TAccountId, TAccountKeys> {
	pub account_id: TAccountId,
	pub account_keys: TAccountKeys,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord)]
pub enum Candidate<TAccountId, TAccountKeys> {
	Permissioned(PermissionedCandidate<TAccountId, TAccountKeys>),
	Registered(CandidateWithStake<TAccountId, TAccountKeys>),
}

impl<AuthorityId, AuthorityKeys> From<Candidate<AuthorityId, AuthorityKeys>>
	for CommitteeMember<AuthorityId, AuthorityKeys>
{
	fn from(candidate: Candidate<AuthorityId, AuthorityKeys>) -> Self {
		match candidate {
			Candidate::Permissioned(member) => {
				Self::Permissioned { id: member.account_id, keys: member.account_keys }
			},
			Candidate::Registered(member) => Self::Registered {
				id: member.account_id,
				keys: member.account_keys,
				stake_pool_pub_key: member.stake_pool_pub_key,
			},
		}
	}
}

impl<TAccountId, TAccountKeys> Candidate<TAccountId, TAccountKeys> {
	pub fn account_id(&self) -> &TAccountId {
		match self {
			Candidate::Permissioned(c) => &c.account_id,
			Candidate::Registered(c) => &c.account_id,
		}
	}

	pub fn account_keys(&self) -> &TAccountKeys {
		match self {
			Candidate::Permissioned(c) => &c.account_keys,
			Candidate::Registered(c) => &c.account_keys,
		}
	}

	pub fn stake_delegation(&self) -> Option<StakeDelegation> {
		match self {
			Candidate::Permissioned(_) => None,
			Candidate::Registered(c) => Some(c.stake_delegation),
		}
	}
}

/// Get the valid trustless candidates from the registrations from inherent data
pub fn filter_trustless_candidates_registrations<TAccountId, TAccountKeys>(
	candidate_registrations: Vec<CandidateRegistrations>,
	genesis_utxo: UtxoId,
) -> Vec<(Candidate<TAccountId, TAccountKeys>, selection::Weight)>
where
	TAccountKeys: From<(sr25519::Public, ed25519::Public)>,
	TAccountId: From<ecdsa::Public>,
{
	candidate_registrations
		.into_iter()
		.flat_map(|candidate_registrations| {
			select_latest_valid_candidate(candidate_registrations, genesis_utxo)
		})
		.map(|c| {
			let weight = c.stake_delegation.0.into();
			(Candidate::Registered(c), weight)
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
			Some(Candidate::Permissioned(PermissionedCandidate { account_id, account_keys }))
		})
		.collect()
}

fn select_latest_valid_candidate<TAccountId, TAccountKeys>(
	candidate_registrations: CandidateRegistrations,
	genesis_utxo: UtxoId,
) -> Option<CandidateWithStake<TAccountId, TAccountKeys>>
where
	TAccountId: From<ecdsa::Public>,
	TAccountKeys: From<(sr25519::Public, ed25519::Public)>,
{
	let stake_delegation = validate_stake(candidate_registrations.stake_delegation).ok()?;
	let stake_pool_pub_key = candidate_registrations.stake_pool_public_key;

	let ((account_id, account_keys), _) = candidate_registrations
		.registrations
		.into_iter()
		.filter_map(|registration_data| {
			match validate_registration_data(&stake_pool_pub_key, &registration_data, genesis_utxo)
			{
				Ok(candidate) => Some((candidate, registration_data.utxo_info)),
				Err(_) => None,
			}
		})
		// Get the latest valid registration of the authority candidate
		.max_by_key(|(_, utxo_info)| utxo_info.ordering_key())?;

	Some(CandidateWithStake {
		account_id: account_id.into(),
		account_keys: account_keys.into(),
		stake_delegation,
		stake_pool_pub_key,
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
pub fn validate_registration_data(
	stake_pool_pub_key: &StakePoolPublicKey,
	registration_data: &RegistrationData,
	genesis_utxo: UtxoId,
) -> Result<(ecdsa::Public, (sr25519::Public, ed25519::Public)), RegistrationDataError> {
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
		genesis_utxo,
		sidechain_pub_key: registration_data.sidechain_pub_key.0.clone(),
		registration_utxo: registration_data.registration_utxo,
	};

	let signed_message_encoded = minicbor::to_vec(signed_message.to_datum())
		.expect("`RegisterValidatorSignedMessage` should always be encodable");

	verify_stake_pool_signature(stake_pool_pub_key, registration_data, &signed_message_encoded)?;
	verify_sidechain_signature(
		sidechain_pub_key,
		&registration_data.sidechain_signature,
		&signed_message_encoded,
	)?;
	verify_tx_inputs(registration_data)?;

	// TODO - Stake Validation: https://input-output.atlassian.net/browse/ETCM-4082

	Ok((sidechain_pub_key, (aura_pub_key, grandpa_pub_key)))
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

fn verify_stake_pool_signature(
	stake_pool_pub_key: &StakePoolPublicKey,
	registration_data: &RegistrationData,
	signed_message_encoded: &[u8],
) -> Result<(), RegistrationDataError> {
	let spo_signature: [u8; 64] = registration_data
		.mainchain_signature
		.0
		.try_into()
		.map_err(|_| RegistrationDataError::InvalidMainchainSignature)?;
	let spo_signature = ed25519::Signature::from(spo_signature);
	if spo_signature.verify(signed_message_encoded, &ed25519::Public::from(stake_pool_pub_key.0)) {
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
	if registration_data.tx_inputs.contains(&registration_data.registration_utxo) {
		Ok(())
	} else {
		Err(RegistrationDataError::InvalidTxInput)
	}
}

// When implementing, make sure that the same validation is used here and in the committee selection logic
sp_api::decl_runtime_apis! {
	pub trait CandidateValidationApi {
		fn validate_registered_candidate_data(mainchain_pub_key: &StakePoolPublicKey, registration_data: &RegistrationData) -> Option<RegistrationDataError>;
		fn validate_stake(stake: Option<StakeDelegation>) -> Option<StakeError>;
		fn validate_permissioned_candidate_data(candidate: PermissionedCandidateData) -> Option<PermissionedCandidateDataError>;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tests::{AccountId, AccountKeys};
	use hex_literal::hex;
	use sp_core::Pair;

	/// Get Valid Parameters of the `is_registration_data_valid()` function
	fn create_valid_parameters() -> (StakePoolPublicKey, RegistrationData, UtxoId) {
		let registration_utxo = UtxoId {
			tx_hash: McTxHash(hex!(
				"d260a76b267e27fdf79c217ec61b776d6436dc78eefeac8f3c615486a71f38eb"
			)),
			index: UtxoIndex(1),
		};

		let registration_data = RegistrationData {
			registration_utxo,
			sidechain_signature: SidechainSignature(
				hex!("f3622ed6e121836765f684068ecf3cf13eb3eb7d2fc7edcabdb41cad940434ca7b9edf45ce8b6d8d2b2a842fb8265856a3f950d72d42499c72ad28dc46b5dc90").to_vec()
			),
			mainchain_signature: MainchainSignature(
				hex!("2e5e39928409aa9ef5ab955da1cd3819ebd2e7461e766d685305280b6986929e3df2bf4cc5a2ed355c20a0dfb44f2e6ef1f36b6da3bbae3ff0c040705b412e07")
			),
			cross_chain_signature: CrossChainSignature(
				hex!("f3622ed6e121836765f684068ecf3cf13eb3eb7d2fc7edcabdb41cad940434ca7b9edf45ce8b6d8d2b2a842fb8265856a3f950d72d42499c72ad28dc46b5dc90").to_vec()
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
			tx_inputs: vec![registration_utxo],
		};

		let genesis_utxo = UtxoId {
			tx_hash: McTxHash(hex!(
				"f17e6d3aa72095e04489d13d776bf05a66b5a8c49d89397c28b18a1784b9950e"
			)),
			index: UtxoIndex(0),
		};

		(
			StakePoolPublicKey(hex!(
				"fb335cabe7d3dd77d0177cd332e9a44998d9d5085b811650853b7bb0752a8bef"
			)),
			registration_data,
			genesis_utxo,
		)
	}

	mod is_registration_data_valid_tests {
		use super::*;
		use core::str::FromStr;
		use sidechain_domain::{
			CrossChainPublicKey, McTxHash, SidechainPublicKey, SidechainSignature,
		};
		use sp_core::Pair;

		fn create_parameters(
			signing_sidechain_account: ecdsa::Pair,
			sidechain_pub_key: Vec<u8>,
		) -> (StakePoolPublicKey, RegistrationData, UtxoId) {
			let mainchain_account = ed25519::Pair::from_seed_slice(&[7u8; 32]).unwrap();
			let genesis_utxo =
				UtxoId { tx_hash: McTxHash([7u8; TX_HASH_SIZE]), index: UtxoIndex(0) };
			let signed_message = RegisterValidatorSignedMessage {
				genesis_utxo,
				sidechain_pub_key: sidechain_pub_key.clone(),
				registration_utxo: genesis_utxo,
			};

			let signed_message_encoded = minicbor::to_vec(signed_message.to_datum()).unwrap();

			let mainchain_signature = mainchain_account.sign(&signed_message_encoded[..]);
			let sidechain_signature =
				signing_sidechain_account.sign(&signed_message_encoded[..]).0.as_slice()[..64]
					.to_vec();

			let registration_data = RegistrationData {
				registration_utxo: signed_message.registration_utxo,
				sidechain_signature: SidechainSignature(sidechain_signature),
				mainchain_signature: MainchainSignature(mainchain_signature.0),
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
				tx_inputs: vec![signed_message.registration_utxo],
			};

			(StakePoolPublicKey(mainchain_account.public().0), registration_data, genesis_utxo)
		}

		#[test]
		fn should_work() {
			let (mainchain_pub_key, registration_data, genesis_utxo) = create_valid_parameters();
			assert!(validate_registration_data(
				&mainchain_pub_key,
				&registration_data,
				genesis_utxo,
			)
			.is_ok());
		}

		#[test]
		fn should_not_work_if_mainchain_pub_key_is_different() {
			let (mainchain_pub_key, registration_data, genesis_utxo) = create_valid_parameters();
			let different_mainchain_pub_key =
				StakePoolPublicKey(ed25519::Pair::from_seed_slice(&[0u8; 32]).unwrap().public().0);
			assert_ne!(mainchain_pub_key, different_mainchain_pub_key);
			assert_eq!(
				validate_registration_data(
					&different_mainchain_pub_key,
					&registration_data,
					genesis_utxo,
				),
				Err(RegistrationDataError::InvalidMainchainSignature)
			);
		}

		#[test]
		fn should_not_work_if_sidechain_pub_key_is_different() {
			let signing_sidechain_account = ecdsa::Pair::from_seed_slice(&[77u8; 32]).unwrap();
			let sidechain_pub_key =
				ecdsa::Pair::from_seed_slice(&[123u8; 32]).unwrap().public().0.to_vec();
			let (mainchain_pub_key, registration_data, genesis_utxo) =
				create_parameters(signing_sidechain_account, sidechain_pub_key);
			assert_eq!(
				validate_registration_data(&mainchain_pub_key, &registration_data, genesis_utxo,),
				Err(RegistrationDataError::InvalidSidechainSignature)
			);
		}

		#[test]
		fn should_fail_validation_for_invalid_grandpa_key() {
			let signing_sidechain_account = ecdsa::Pair::from_seed_slice(&[77u8; 32]).unwrap();
			let sidechain_pub_key =
				ecdsa::Pair::from_seed_slice(&[123u8; 32]).unwrap().public().0.to_vec();
			let (mainchain_pub_key, mut registration_data, genesis_utxo) =
				create_parameters(signing_sidechain_account, sidechain_pub_key);
			registration_data.grandpa_pub_key = GrandpaPublicKey(vec![3; 4]);
			assert_eq!(
				validate_registration_data(&mainchain_pub_key, &registration_data, genesis_utxo,),
				Err(RegistrationDataError::InvalidGrandpaKey)
			);
		}

		#[test]
		fn should_fail_validation_for_invalid_aura_key() {
			let signing_sidechain_account = ecdsa::Pair::from_seed_slice(&[77u8; 32]).unwrap();
			let sidechain_pub_key =
				ecdsa::Pair::from_seed_slice(&[123u8; 32]).unwrap().public().0.to_vec();
			let (mainchain_pub_key, mut registration_data, genesis_utxo) =
				create_parameters(signing_sidechain_account, sidechain_pub_key);
			registration_data.aura_pub_key = AuraPublicKey(vec![3; 4]);
			assert_eq!(
				validate_registration_data(&mainchain_pub_key, &registration_data, genesis_utxo),
				Err(RegistrationDataError::InvalidAuraKey)
			);
		}

		#[test]
		fn should_not_work_if_sidechain_params_is_different() {
			let (mainchain_pub_key, registration_data, genesis_utxo) = create_valid_parameters();
			let different_genesis_utxo = UtxoId::from_str(
				"ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00#7",
			)
			.unwrap();
			assert_ne!(different_genesis_utxo, genesis_utxo);
			assert!(validate_registration_data(
				&mainchain_pub_key,
				&registration_data,
				different_genesis_utxo,
			)
			.is_err());
		}

		#[test]
		fn should_not_work_if_tx_input_is_invalid() {
			let (mainchain_pub_key, mut registration_data, genesis_utxo) =
				create_valid_parameters();
			registration_data.tx_inputs = vec![];
			assert_eq!(
				validate_registration_data(&mainchain_pub_key, &registration_data, genesis_utxo,),
				Err(RegistrationDataError::InvalidTxInput)
			);
		}
	}

	#[test]
	fn should_filter_out_candidates_with_invalid_stake() {
		let (mc_pub_key, registration_data, genesis_utxo) = create_valid_parameters();
		let candidate_registrations = vec![
			CandidateRegistrations {
				stake_pool_public_key: mc_pub_key.clone(),
				registrations: vec![registration_data.clone()],
				stake_delegation: Some(StakeDelegation(0)),
			},
			CandidateRegistrations {
				stake_pool_public_key: mc_pub_key.clone(),
				registrations: vec![registration_data.clone()],
				stake_delegation: Some(StakeDelegation(1)),
			},
			CandidateRegistrations {
				stake_pool_public_key: mc_pub_key.clone(),
				registrations: vec![registration_data.clone()],
				stake_delegation: None,
			},
			CandidateRegistrations {
				stake_pool_public_key: mc_pub_key,
				registrations: vec![registration_data],
				stake_delegation: Some(StakeDelegation(2)),
			},
		];

		let valid_candidates = filter_trustless_candidates_registrations::<AccountId, AccountKeys>(
			candidate_registrations,
			genesis_utxo,
		);

		assert_eq!(valid_candidates.len(), 2);
		assert_eq!(valid_candidates[0].0.stake_delegation(), Some(StakeDelegation(1)));
		assert_eq!(valid_candidates[1].0.stake_delegation(), Some(StakeDelegation(2)));
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
			valid_candidates.first().unwrap().account_id(),
			&valid_sidechain_pub_key.try_into().unwrap()
		);
		assert_eq!(
			valid_candidates.first().unwrap().account_keys(),
			&(
				valid_candidate.aura_public_key.try_into_sr25519().unwrap(),
				valid_candidate.grandpa_public_key.try_into_ed25519().unwrap()
			)
				.into()
		);
	}
}
