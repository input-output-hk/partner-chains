//! Functionality related to filtering invalid candidates from the candidates

use crate::{CommitteeMember, MaybeFromCandidateKeys};
use frame_support::pallet_prelude::TypeInfo;
use parity_scale_codec::{Decode, Encode};
use plutus::*;
use plutus_datum_derive::ToDatum;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sidechain_domain::*;
use sp_core::{ecdsa, ed25519};
use sp_runtime::traits::Verify;

/// Signed Message of the Authority Candidate to register
/// It's ToDatum implementation has to produce datum that has to match main chain structure,
/// because it will be hashed and signed for signature verification.
#[derive(Debug, ToDatum)]
pub struct RegisterValidatorSignedMessage {
	/// Genesis UTxO identifying the Partner Chain
	pub genesis_utxo: UtxoId,
	/// Partner Chain public key of the registered candidate
	pub sidechain_pub_key: Vec<u8>,
	/// UTxO that is an input parameter to the registration transaction.
	/// It is spent during the registration process. Prevents replay attacks.
	pub registration_utxo: UtxoId,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord)]
/// Type representing a registered candidate.
pub struct CandidateWithStake<TAccountId, TAccountKeys> {
	/// Stake pool public key of the registered candidate
	pub stake_pool_pub_key: StakePoolPublicKey,
	/// Amount of ADA staked/locked by the Authority Candidate
	pub stake_delegation: StakeDelegation,
	/// Account id of the registered candidate
	pub account_id: TAccountId,
	/// Account keys of the registered candidate
	pub account_keys: TAccountKeys,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord)]
/// Type representing a permissioned candidate.
pub struct PermissionedCandidate<TAccountId, TAccountKeys> {
	/// Account id of the permissioned candidate
	pub account_id: TAccountId,
	/// Account keys of the permissioned candidate
	pub account_keys: TAccountKeys,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord)]
/// Type representing candidates. Candidates can be either registered or permissioned.
/// See [sidechain_domain::DParameter] for more details.
pub enum Candidate<TAccountId, TAccountKeys> {
	/// A permissioned candidate
	Permissioned(PermissionedCandidate<TAccountId, TAccountKeys>),
	/// A registered candidate
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
	/// Returns the account id of the candidate
	pub fn account_id(&self) -> &TAccountId {
		match self {
			Candidate::Permissioned(c) => &c.account_id,
			Candidate::Registered(c) => &c.account_id,
		}
	}

	/// Returns the account keys of the candidate
	pub fn account_keys(&self) -> &TAccountKeys {
		match self {
			Candidate::Permissioned(c) => &c.account_keys,
			Candidate::Registered(c) => &c.account_keys,
		}
	}

	/// Returns the stake delegation of the candidate
	pub fn stake_delegation(&self) -> Option<StakeDelegation> {
		match self {
			Candidate::Permissioned(_) => None,
			Candidate::Registered(c) => Some(c.stake_delegation),
		}
	}
}

/// Get the valid trustless candidates from the registrations from inherent data
pub fn filter_trustless_candidates_registrations<TAccountId, TAccountKeys: MaybeFromCandidateKeys>(
	candidate_registrations: Vec<CandidateRegistrations>,
	genesis_utxo: UtxoId,
) -> Vec<(Candidate<TAccountId, TAccountKeys>, selection::Weight)>
where
	TAccountId: From<ecdsa::Public>,
{
	candidate_registrations
		.into_iter()
		.flat_map(|candidate_registrations| {
			select_latest_valid_candidate::<TAccountId, TAccountKeys>(
				candidate_registrations,
				genesis_utxo,
			)
		})
		.map(|c| {
			let weight = c.stake_delegation.0.into();
			(Candidate::Registered(c), weight)
		})
		.collect()
}
/// Filters invalid candidates from a list of [PermissionedCandidateData].
pub fn filter_invalid_permissioned_candidates<TAccountId, TAccountKeys: MaybeFromCandidateKeys>(
	permissioned_candidates: Vec<PermissionedCandidateData>,
) -> Vec<Candidate<TAccountId, TAccountKeys>>
where
	TAccountId: From<ecdsa::Public>,
{
	permissioned_candidates
		.into_iter()
		.filter_map(|candidate| {
			let (partner_chain_key, account_keys) =
				validate_permissioned_candidate_data::<TAccountKeys>(candidate).ok()?;
			let account_id = partner_chain_key.into();
			Some(Candidate::Permissioned(PermissionedCandidate { account_id, account_keys }))
		})
		.collect()
}

fn select_latest_valid_candidate<TAccountId, TAccountKeys: MaybeFromCandidateKeys>(
	candidate_registrations: CandidateRegistrations,
	genesis_utxo: UtxoId,
) -> Option<CandidateWithStake<TAccountId, TAccountKeys>>
where
	TAccountId: From<ecdsa::Public>,
{
	let stake_delegation = validate_stake(candidate_registrations.stake_delegation).ok()?;
	let stake_pool_pub_key = candidate_registrations.stake_pool_public_key;

	let ((account_id, account_keys), _) = candidate_registrations
		.registrations
		.into_iter()
		.filter_map(|registration_data| {
			match validate_registration_data::<TAccountKeys>(
				&stake_pool_pub_key,
				&registration_data,
				genesis_utxo,
			) {
				Ok(candidate) => Some((candidate, registration_data.utxo_info)),
				Err(_) => None,
			}
		})
		// Get the latest valid registration of the authority candidate
		.max_by_key(|(_, utxo_info)| utxo_info.ordering_key())?;

	Some(CandidateWithStake {
		account_id: account_id.into(),
		account_keys,
		stake_delegation,
		stake_pool_pub_key,
	})
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(thiserror::Error, Serialize, Deserialize))]
/// Stake validation error type
pub enum StakeError {
	/// Stake should be greater than 0.
	#[cfg_attr(feature = "std", error("Stake should be greater than 0"))]
	InvalidStake,
	/// Stake delegation information cannot be computed yet. Registration will turn valid if stake delegation for the epoch will be greater than 0.
	#[cfg_attr(
		feature = "std",
		error(
			"Stake delegation information cannot be computed yet. Registration will turn valid if stake delegation for the epoch will be greater than 0"
		)
	)]
	UnknownStake,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(thiserror::Error, Serialize, Deserialize))]
/// Registration data error type
pub enum RegistrationDataError {
	/// Registration with invalid mainchain signature
	#[cfg_attr(feature = "std", error("Registration with invalid mainchain signature"))]
	InvalidMainchainSignature,
	/// Registration with invalid sidechain signature
	#[cfg_attr(feature = "std", error("Registration with invalid sidechain signature"))]
	InvalidSidechainSignature,
	/// Registration with invalid transaction input
	#[cfg_attr(feature = "std", error("Registration with invalid transaction input"))]
	InvalidTxInput,
	/// Registration with invalid mainchain public key
	#[cfg_attr(feature = "std", error("Registration with invalid mainchain public key"))]
	InvalidMainchainPubKey,
	/// Registration with invalid sidechain public key
	#[cfg_attr(feature = "std", error("Registration with invalid sidechain public key"))]
	InvalidSidechainPubKey,
	/// Registration with invalid Account keys
	#[cfg_attr(feature = "std", error("Registration with invalid account keys"))]
	InvalidAccountKeys,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(thiserror::Error, Serialize, Deserialize))]
/// Permissioned candidate data error type
pub enum PermissionedCandidateDataError {
	/// Permissioned candidate with invalid sidechain public key
	#[cfg_attr(feature = "std", error("Permissioned candidate with invalid sidechain public key"))]
	InvalidSidechainPubKey,
	/// Permissioned candidate with invalid account keys
	#[cfg_attr(feature = "std", error("Permissioned candidate with invalid account keys"))]
	InvalidAccountKeys,
}

/// Validates Account keys and Partner Chain public keys of [PermissionedCandidateData].
pub fn validate_permissioned_candidate_data<TAccountKeys: MaybeFromCandidateKeys>(
	candidate: PermissionedCandidateData,
) -> Result<(ecdsa::Public, TAccountKeys), PermissionedCandidateDataError> {
	let ecdsa_bytes: [u8; 33] = candidate
		.sidechain_public_key
		.0
		.try_into()
		.map_err(|_| PermissionedCandidateDataError::InvalidSidechainPubKey)?;
	Ok((
		ecdsa_bytes.into(),
		MaybeFromCandidateKeys::maybe_from(&candidate.keys)
			.ok_or_else(|| PermissionedCandidateDataError::InvalidAccountKeys)?,
	))
}

/// Validates registration data provided by the authority candidate.
///
/// Validates:
/// * Account keys and Partner Chain public keys of the candidate
/// * stake pool signature
/// * sidechain signature
/// * transaction inputs contain correct registration utxo
pub fn validate_registration_data<TAccountKeys: MaybeFromCandidateKeys>(
	stake_pool_pub_key: &StakePoolPublicKey,
	registration_data: &RegistrationData,
	genesis_utxo: UtxoId,
) -> Result<(ecdsa::Public, TAccountKeys), RegistrationDataError> {
	let account_keys = MaybeFromCandidateKeys::maybe_from(&registration_data.keys)
		.ok_or(RegistrationDataError::InvalidAccountKeys)?;
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

	Ok((sidechain_pub_key, account_keys))
}

/// Validates stake delegation. Stake must be known and positive.
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
	// Pre EIP155 signature.v convention
	const NEGATIVE_POINT_SIGN: u8 = 27;
	const POSITIVE_POINT_SIGN: u8 = 28;
	let is_valid = [NEGATIVE_POINT_SIGN, POSITIVE_POINT_SIGN].iter().any(|v| {
		sidechain_signature_with_v[64] = *v;
		ecdsa::Signature::from(sidechain_signature_with_v).verify(signed_message_encoded, &pub_key)
	});
	if is_valid { Ok(()) } else { Err(RegistrationDataError::InvalidSidechainSignature) }
}

fn verify_tx_inputs(registration_data: &RegistrationData) -> Result<(), RegistrationDataError> {
	if registration_data.tx_inputs.contains(&registration_data.registration_utxo) {
		Ok(())
	} else {
		Err(RegistrationDataError::InvalidTxInput)
	}
}

sp_api::decl_runtime_apis! {
	/// Runtime API trait for candidate validation
	///
	/// When implementing, make sure that the same validation is used here and in the committee selection logic!
	pub trait CandidateValidationApi {
		/// Should validate data provided by registered candidate,
		/// and return [RegistrationDataError] in case of validation failure.
		///
		/// Should validate:
		/// * Account keys and Partner Chain public keys of the candidate
		/// * stake pool signature
		/// * sidechain signature
		/// * transaction inputs contain correct registration utxo
		fn validate_registered_candidate_data(mainchain_pub_key: &StakePoolPublicKey, registration_data: &RegistrationData) -> Option<RegistrationDataError>;
		/// Should validate candidate stake and return [StakeError] in case of validation failure.
		/// Should validate stake exists and is positive.
		fn validate_stake(stake: Option<StakeDelegation>) -> Option<StakeError>;
		/// Should validate data provided by permissioned candidate,
		/// and return [PermissionedCandidateDataError] in case of validation failure.
		///
		/// Should validate:
		/// * Account keys and Partner Chain public keys of the candidate
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
			keys: CandidateKeys(vec![
				CandidateKey{ id: *b"sr25", bytes: hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d").to_vec()},
				CandidateKey{ id: *b"ed25", bytes: hex!("88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee").to_vec()},
			]),
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
		use sp_runtime::key_types::{AURA, GRANDPA};

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
				keys: CandidateKeys(vec![
					CandidateKey { id: *b"sr25", bytes: [1u8; 32].to_vec() },
					CandidateKey { id: *b"ed25", bytes: [2u8; 32].to_vec() },
				]),
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
			assert!(
				validate_registration_data::<AccountKeys>(
					&mainchain_pub_key,
					&registration_data,
					genesis_utxo,
				)
				.is_ok()
			);
		}

		#[test]
		fn should_not_work_if_mainchain_pub_key_is_different() {
			let (mainchain_pub_key, registration_data, genesis_utxo) = create_valid_parameters();
			let different_mainchain_pub_key =
				StakePoolPublicKey(ed25519::Pair::from_seed_slice(&[0u8; 32]).unwrap().public().0);
			assert_ne!(mainchain_pub_key, different_mainchain_pub_key);
			assert_eq!(
				validate_registration_data::<AccountKeys>(
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
				validate_registration_data::<AccountKeys>(
					&mainchain_pub_key,
					&registration_data,
					genesis_utxo,
				),
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
			registration_data.keys = CandidateKeys(vec![
				AuraPublicKey(vec![1; 32]).into(),
				GrandpaPublicKey(vec![3; 4]).into(),
			]);
			assert_eq!(
				validate_registration_data::<AccountKeys>(
					&mainchain_pub_key,
					&registration_data,
					genesis_utxo,
				),
				Err(RegistrationDataError::InvalidAccountKeys)
			);
		}

		#[test]
		fn should_fail_validation_for_invalid_aura_key() {
			let signing_sidechain_account = ecdsa::Pair::from_seed_slice(&[77u8; 32]).unwrap();
			let sidechain_pub_key =
				ecdsa::Pair::from_seed_slice(&[123u8; 32]).unwrap().public().0.to_vec();
			let (mainchain_pub_key, mut registration_data, genesis_utxo) =
				create_parameters(signing_sidechain_account, sidechain_pub_key);
			registration_data.keys = CandidateKeys(vec![
				CandidateKey::new(AURA, vec![3; 4]),
				CandidateKey::new(GRANDPA, vec![2; 32]),
			]);
			assert_eq!(
				validate_registration_data::<AccountKeys>(
					&mainchain_pub_key,
					&registration_data,
					genesis_utxo
				),
				Err(RegistrationDataError::InvalidAccountKeys)
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
			assert!(
				validate_registration_data::<AccountKeys>(
					&mainchain_pub_key,
					&registration_data,
					different_genesis_utxo,
				)
				.is_err()
			);
		}

		#[test]
		fn should_not_work_if_tx_input_is_invalid() {
			let (mainchain_pub_key, mut registration_data, genesis_utxo) =
				create_valid_parameters();
			registration_data.tx_inputs = vec![];
			assert_eq!(
				validate_registration_data::<AccountKeys>(
					&mainchain_pub_key,
					&registration_data,
					genesis_utxo,
				),
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
		let aura_key_bytes =
			hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d");
		let valid_aura_key = CandidateKey { id: *b"sr25", bytes: aura_key_bytes.to_vec() };
		let grandpa_key_bytes =
			hex!("88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee");
		let valid_grandpa_key = CandidateKey { id: *b"ed25", bytes: grandpa_key_bytes.to_vec() };
		let valid_candidate = PermissionedCandidateData {
			sidechain_public_key: valid_sidechain_pub_key.clone(),
			keys: CandidateKeys(vec![
				valid_aura_key.clone().into(),
				valid_grandpa_key.clone().into(),
			]),
		};
		let permissioned_candidates = vec![
			PermissionedCandidateData {
				keys: CandidateKeys(vec![
					AuraPublicKey(vec![1, 2]).into(),
					valid_grandpa_key.clone().into(),
				]),
				..valid_candidate.clone()
			},
			PermissionedCandidateData {
				sidechain_public_key: SidechainPublicKey(vec![2; 2]),
				..valid_candidate.clone()
			},
			PermissionedCandidateData {
				keys: CandidateKeys(vec![
					valid_aura_key.clone().into(),
					GrandpaPublicKey(vec![3; 4]).into(),
				]),
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
			&AccountKeys {
				aura: sp_core::sr25519::Public::from(aura_key_bytes).into(),
				grandpa: sp_core::ed25519::Public::from(grandpa_key_bytes).into(),
			}
		);
	}
}
