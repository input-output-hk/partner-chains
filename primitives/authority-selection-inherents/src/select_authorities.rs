//! Functionality related to selecting the validators from the valid candidates

use crate::authority_selection_inputs::AuthoritySelectionInputs;
use crate::filter_invalid_candidates::{
	filter_invalid_permissioned_candidates, filter_trustless_candidates_registrations, Candidate,
	CandidateWithStake,
};
use frame_support::BoundedVec;
use log::{info, warn};
use plutus::*;
use selection::{Weight, WeightedRandomSelectionConfig};
use sidechain_domain::{DParameter, ScEpochNumber};
use sp_core::{ecdsa, ed25519, sr25519, Get};

type CandidateWithWeight<A, B> = (Candidate<A, B>, Weight);

/// Pseudo-random selection the authorities for the given sidechain epoch, according to the
/// Ariadne specification: https://input-output.atlassian.net/wiki/spaces/SID/pages/4228612151/Ariadne+-+committee+selection+algorithm
///
/// Seed is constructed from the MC epoch nonce and the sidechain epoch.
///
/// Committee size is P+T, where P (permissioned) and T (trustless) are constituents of the D parameter.
///
/// Committee is a result of the weighted selection with repetition.
/// Weight function for trustless candidate is:
///   * let `n` be the number of permissioned candidates from MC data
///   * if `n == 0`, then the weight is `stake_delegation`
///   * otherwise, the weight is `n * T * stake_delegation`
///Weight for each permissioned candidates is:
///   * let `W` be the sum of all stake delegations of trustless candidates
///   * if `W == 0` or `T == 0` (there are no valid trustless candidates, or they are not taken into account), then the weight is `1`
///   * otherwise, the weight is `P * W`
pub fn select_authorities<
	TAccountId: Clone + Ord + TryFrom<sidechain_domain::SidechainPublicKey> + From<ecdsa::Public>,
	TAccountKeys: Clone + From<(sr25519::Public, ed25519::Public)>,
	Params: Clone + ToDatum,
	MaxValidators: Get<u32>,
>(
	sidechain_params: Params,
	input: AuthoritySelectionInputs,
	sidechain_epoch: ScEpochNumber,
) -> Option<BoundedVec<(TAccountId, TAccountKeys), MaxValidators>> {
	let valid_trustless_candidates = filter_trustless_candidates_registrations::<
		TAccountId,
		TAccountKeys,
		Params,
	>(input.registered_candidates, sidechain_params);
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
	candidates_with_weight.sort_by(|a, b| a.0.account_id.cmp(&b.0.account_id));

	let random_seed =
		selection::impls::seed_from_nonce_and_sc_epoch(&input.epoch_nonce, &sidechain_epoch);
	let committee_size =
		input.d_parameter.num_registered_candidates + input.d_parameter.num_permissioned_candidates;
	if let Some(validators) =
		weighted_selection(candidates_with_weight, committee_size, random_seed)
	{
		info!("💼 Selected {} validators for epoch {}, from {} permissioned candidates and {} trustless candidates", validators.len(), sidechain_epoch, valid_permissioned_candidates.len(), valid_trustless_candidates.len());
		Some(BoundedVec::truncate_from(validators))
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
		.map(|c| (c.candidate.clone(), u128::from(c.stake_delegation.0) * weight_factor))
		.collect()
}

fn permissioned_candidates_with_weights<A: Clone, B: Clone>(
	permissioned_candidates: &[Candidate<A, B>],
	d_parameter: &DParameter,
	valid_trustless_candidates: &[CandidateWithStake<A, B>],
) -> Vec<CandidateWithWeight<A, B>> {
	let total_stake: u64 = valid_trustless_candidates.iter().map(|c| c.stake_delegation.0).sum();
	let weight = if total_stake > 0 && d_parameter.num_registered_candidates > 0 {
		u128::from(d_parameter.num_permissioned_candidates) * u128::from(total_stake)
	} else {
		1 // if there are no trustless candidates, permissioned candidates should be selected with equal weight
	};
	permissioned_candidates.iter().map(|c| (c.clone(), weight)).collect::<Vec<_>>()
}

fn weighted_selection<TAccountId: Clone + Ord, TAccountKeys: Clone>(
	candidates: Vec<CandidateWithWeight<TAccountId, TAccountKeys>>,
	size: u16,
	random_seed: [u8; 32],
) -> Option<Vec<(TAccountId, TAccountKeys)>> {
	Some(
		WeightedRandomSelectionConfig { size }
			.select_authorities(candidates, random_seed)?
			.into_iter()
			.map(|c| (c.account_id, c.account_keys))
			.collect(),
	)
}
