use crate::{weighted_random, Weight, WeightedRandomSelectionConfig};
use alloc::vec::Vec;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

pub trait Candidate {
	type CandidateId: Ord;
	fn candidate_id(&self) -> Self::CandidateId;
}

pub fn select_authorities<SC>(
	num_registered_candidates: u16,
	num_permissioned_candidates: u16,
	registered_candidates: Vec<(SC, Weight)>,
	permissioned_candidates: Vec<SC>,
	seed: <ChaCha20Rng as SeedableRng>::Seed,
) -> Option<Vec<SC>>
where
	SC: Candidate + Clone,
{
	let committee_size = num_registered_candidates + num_permissioned_candidates;
	let weighted_config = WeightedRandomSelectionConfig { size: committee_size };
	let mut weighted_candidates = alloc::vec![];

	let total_stake: u128 = registered_candidates.iter().map(|(_, weight)| weight).sum();

	let registered_candidates = registered_candidates_with_weights(
		&registered_candidates,
		num_registered_candidates,
		permissioned_candidates.len(),
	);

	let permissioned_candidates = permissioned_candidates_with_weights(
		&permissioned_candidates,
		num_permissioned_candidates,
		num_registered_candidates,
		total_stake,
	);

	weighted_candidates.extend(registered_candidates);
	weighted_candidates.extend(permissioned_candidates);
	weighted_candidates.sort_by_key(|(c, _)| c.candidate_id());

	weighted_random::select_authorities(weighted_candidates, seed, &weighted_config)
}

fn registered_candidates_with_weights<Candidate: Clone>(
	registered_candidates: &[(Candidate, Weight)],
	num_registered_candidates: u16,
	permissioned_candidates_count: usize,
) -> Vec<(Candidate, Weight)> {
	let weight_factor = if permissioned_candidates_count > 0 {
		u128::from(num_registered_candidates) * permissioned_candidates_count as u128
	} else {
		1 // if there are no permissioned candidates, registered candidates should be selected using unmodified stake
	};
	registered_candidates
		.iter()
		.map(|(c, weight)| (c.clone().into(), weight * weight_factor))
		.collect()
}

fn permissioned_candidates_with_weights<Candidate: Clone>(
	permissioned_candidates: &[Candidate],
	num_permissioned_candidates: u16,
	num_registered_candidates: u16,
	total_stake: u128,
) -> Vec<(Candidate, Weight)> {
	let weight = if total_stake > 0 && num_registered_candidates > 0 {
		u128::from(num_permissioned_candidates) * u128::from(total_stake)
	} else {
		1 // if there are no registered candidates, permissioned candidates should be selected with equal weight
	};
	permissioned_candidates.iter().map(|c| (c.clone().into(), weight)).collect()
}
