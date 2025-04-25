extern crate alloc;

use crate::*;
use alloc::vec::Vec;
use rand::Rng;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

/// Simple random weighted selection
///
/// When selecting out of `n` candidates with weights `w_1`, `w_2`, ..., `w_n`, independently assigns each
/// committee seat to the k-th candidate with probability `w_k / (w_1 + w_2 + ... + w_n)`.
pub fn select_authorities<T: Clone>(
	weighted_candidates: Vec<(T, Weight)>,
	seed: <ChaCha20Rng as SeedableRng>::Seed,
	size: u16,
) -> Option<Vec<T>> {
	let size = usize::from(size);
	let total_weight: Weight = weighted_candidates.iter().map(|(_, weight)| weight).sum();

	let mut committee: Vec<T> = alloc::vec![];

	let mut rng = ChaCha20Rng::from_seed(seed);

	while committee.len() < size && !weighted_candidates.is_empty() {
		let selected_index = select_with_weight(&weighted_candidates, total_weight, &mut rng);
		let selected = weighted_candidates[selected_index].0.clone();
		committee.push(selected);
	}

	if size <= committee.len() {
		Some(committee)
	} else {
		None
	}
}

fn select_with_weight<T>(
	candidates: &[(T, Weight)],
	total_weight: Weight,
	rand: &mut ChaCha20Rng,
) -> usize {
	let random_number: u128 = rand.gen_range(0..total_weight);

	let mut cumulative_weight: Weight = 0;
	for (index, (_, weight)) in candidates.iter().enumerate() {
		cumulative_weight += weight;
		if cumulative_weight > random_number {
			return index;
		}
	}

	panic!("Did not select any candidate");
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tests::*;
	use quickcheck_macros::*;

	type CandidatesWithWeights = Vec<(String, Weight)>;

	#[derive(Clone)]
	struct TestWeightedCandidates(CandidatesWithWeights, [u8; 32]);

	fn select<const COMMITTEE_SIZE: u16>(
		candidates: TestWeightedCandidates,
	) -> Option<Vec<String>> {
		select_authorities(candidates.0, candidates.1, COMMITTEE_SIZE)
	}

	fn uniform_weight_candidates(n: u16) -> (Vec<String>, CandidatesWithWeights) {
		let candidates = (0..n)
			.map(|c| "candidate_".to_string() + &c.to_string())
			.collect::<Vec<String>>();
		let with_weights = candidates.iter().cloned().map(|c| (c, 1)).collect();
		(candidates, with_weights)
	}

	const MAX_CANDIDATE_NUMBER: u16 = 1000;

	#[quickcheck]
	fn random_selection_with_repetition(candidate_number: u16, nonce: TestNonce) {
		const COMMITTEE_SIZE: u16 = 2;
		let candidate_number =
			candidate_number % (MAX_CANDIDATE_NUMBER - COMMITTEE_SIZE) + COMMITTEE_SIZE;

		let (candidates, candidates_with_weights) = uniform_weight_candidates(candidate_number);

		let selection_data = TestWeightedCandidates(candidates_with_weights, nonce.0);

		let Some(committee) = select::<COMMITTEE_SIZE>(selection_data) else {
			panic!("select returned a None")
		};

		assert_eq!(committee.len(), COMMITTEE_SIZE as usize);
		assert_subset!(String, committee, candidates);
	}

	#[quickcheck]
	fn random_selection_zero_weight(nonce: TestNonce) {
		let zero = "zero_weight".to_string();
		let non_zero_1 = "non_zero_weight_1".to_string();
		let non_zero_2 = "non_zero_weight_2".to_string();
		let candidates = TestWeightedCandidates(
			vec![(zero, 0), (non_zero_1.clone(), 1), (non_zero_2.clone(), 2)],
			nonce.0,
		);

		let committee = select::<1>(candidates).unwrap();

		assert!(committee == vec![non_zero_1] || committee == vec![non_zero_2]);
	}

	#[quickcheck]
	fn random_selection_cannot_select_from_empty_candidates(nonce: TestNonce) {
		let candidates = TestWeightedCandidates(vec![], nonce.0);

		let committee = select::<1>(candidates);

		assert_eq!(committee, None)
	}

	#[test]
	fn etcm_5304_random_selection_should_not_be_skewed() {
		let mut a_count = 0;
		let candidates = vec![("a".to_string(), u128::MAX / 3), ("b".to_string(), u128::MAX / 3)];
		for i in 0..1000u16 {
			let i_bytes: [u8; 2] = i.to_be_bytes();
			let mut nonce: [u8; 32] = [0u8; 32];
			nonce[0] = i_bytes[0];
			nonce[1] = i_bytes[1];
			let input = TestWeightedCandidates(candidates.clone(), nonce);
			let selected = select::<1>(input).unwrap();
			if selected.contains(&"a".to_string()) {
				a_count += 1;
			}
		}
		assert!(a_count > 470 && a_count < 530)
	}
}
