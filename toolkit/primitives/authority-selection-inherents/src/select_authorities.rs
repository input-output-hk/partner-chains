//! Functionality related to selecting the validators from the valid candidates

use crate::authority_selection_inputs::AuthoritySelectionInputs;
use crate::filter_invalid_candidates::{
	filter_invalid_permissioned_candidates, filter_trustless_candidates_registrations, Candidate,
	CandidateWithStake, PermissionedCandidate,
};
use log::{info, warn};
use plutus::*;
use selection::{Weight, WeightedRandomSelectionConfig};
use sidechain_domain::{DParameter, EpochNonce, ScEpochNumber, UtxoId};
use sp_core::{ecdsa, ed25519, sr25519, U256};

type CandidateWithWeight<A, B> = (Candidate<A, B>, Weight);

/// Pseudo-random selection the authorities for the given sidechain epoch, according to the
/// Ariadne specification: https://input-output.atlassian.net/wiki/spaces/SID/pages/4228612151/Ariadne+-+committee+selection+algorithm
///
/// Seed is constructed from the MC epoch nonce and the sidechain epoch.
///
/// Committee size is P+T, where P (permissioned) and T (trustless) are constituents of the D parameter.
///
/// Committee is a result of the weighted selection with repetition.
///
/// Weight function for trustless candidate is:
///   * let `n` be the number of permissioned candidates from MC data
///   * if `n == 0`, then the weight is `stake_delegation`
///   * otherwise, the weight is `n * T * stake_delegation`
///
/// Weight for each permissioned candidates is:
///   * let `W` be the sum of all stake delegations of trustless candidates
///   * if `W == 0` or `T == 0` (there are no valid trustless candidates, or they are not taken into account), then the weight is `1`
///   * otherwise, the weight is `P * W`
pub fn select_authorities<
	TAccountId: Clone + Ord + TryFrom<sidechain_domain::SidechainPublicKey> + From<ecdsa::Public>,
	TAccountKeys: Clone + From<(sr25519::Public, ed25519::Public)>,
>(
	genesis_utxo: UtxoId,
	input: AuthoritySelectionInputs,
	sidechain_epoch: ScEpochNumber,
) -> Option<Vec<Candidate<TAccountId, TAccountKeys>>> {
	let valid_trustless_candidates = filter_trustless_candidates_registrations::<
		TAccountId,
		TAccountKeys,
	>(input.registered_candidates, genesis_utxo);
	let valid_permissioned_candidates =
		filter_invalid_permissioned_candidates(input.permissioned_candidates);

	let mut candidates_with_weight = trustless_candidates_with_weights(
		&valid_trustless_candidates,
		&input.d_parameter,
		valid_permissioned_candidates.len(),
	);
	candidates_with_weight.extend(permissioned_candidates_with_weights(
		&valid_permissioned_candidates,
		&input.d_parameter,
		&valid_trustless_candidates,
	));
	candidates_with_weight.sort_by(|a, b| a.0.account_id().cmp(&b.0.account_id()));

	let random_seed = seed_from_nonce_and_sc_epoch(&input.epoch_nonce, &sidechain_epoch);
	let committee_size =
		input.d_parameter.num_registered_candidates + input.d_parameter.num_permissioned_candidates;
	if let Some(validators) = (WeightedRandomSelectionConfig { size: committee_size }
		.select_authorities(candidates_with_weight, random_seed))
	{
		info!("💼 Selected committee of {} seats for epoch {} from {} permissioned and {} registered candidates", validators.len(), sidechain_epoch, valid_permissioned_candidates.len(), valid_trustless_candidates.len());
		Some(validators)
	} else {
		warn!("🚫 Failed to select validators for epoch {}", sidechain_epoch);
		None
	}
}

fn trustless_candidates_with_weights<A: Clone, B: Clone>(
	trustless_candidates: &[CandidateWithStake<A, B>],
	d_parameter: &DParameter,
	permissioned_candidates_count: usize,
) -> Vec<CandidateWithWeight<A, B>> {
	let weight_factor = if permissioned_candidates_count > 0 {
		u128::from(d_parameter.num_registered_candidates) * permissioned_candidates_count as u128
	} else {
		1 // if there are no permissioned candidates, trustless candidates should be selected using unmodified stake
	};
	trustless_candidates
		.iter()
		.map(|c| {
			(Candidate::Registered(c.clone()), u128::from(c.stake_delegation.0) * weight_factor)
		})
		.collect()
}

fn permissioned_candidates_with_weights<A: Clone, B: Clone>(
	permissioned_candidates: &[PermissionedCandidate<A, B>],
	d_parameter: &DParameter,
	valid_trustless_candidates: &[CandidateWithStake<A, B>],
) -> Vec<CandidateWithWeight<A, B>> {
	let total_stake: u64 = valid_trustless_candidates.iter().map(|c| c.stake_delegation.0).sum();
	let weight = if total_stake > 0 && d_parameter.num_registered_candidates > 0 {
		u128::from(d_parameter.num_permissioned_candidates) * u128::from(total_stake)
	} else {
		1 // if there are no trustless candidates, permissioned candidates should be selected with equal weight
	};
	permissioned_candidates
		.iter()
		.map(|c| (Candidate::Permissioned(c.clone()), weight))
		.collect::<Vec<_>>()
}

pub fn seed_from_nonce_and_sc_epoch(
	epoch_nonce: &EpochNonce,
	sidechain_epoch_number: &ScEpochNumber,
) -> [u8; 32] {
	let mut epoch_nonce = epoch_nonce.0.clone();
	epoch_nonce.resize_with(32, || 0);
	let epoch_nonce: [u8; 32] =
		epoch_nonce.try_into().expect("Should never fail after being resized");
	let seed_u256: U256 = U256::from_big_endian(&epoch_nonce)
		.overflowing_add(U256::from(sidechain_epoch_number.0))
		.0;
	seed_u256.to_big_endian()
}

#[cfg(test)]
mod tests {
	use super::*;
	use sidechain_domain::{EpochNonce, ScEpochNumber};
	use sp_core::U256;

	#[test]
	fn should_create_correct_seed() {
		let nonce_vec = Vec::from(U256::from(10).to_big_endian());
		assert_eq!(
			seed_from_nonce_and_sc_epoch(&EpochNonce(nonce_vec), &ScEpochNumber(2)),
			U256::from(12).to_big_endian()
		);
	}
}
