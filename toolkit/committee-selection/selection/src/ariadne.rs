use crate::{Weight, WeightedRandomSelectionConfig, weighted_random};
use alloc::vec::Vec;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

/// Pseudo-random selection the authorities for the given sidechain epoch, according to the
/// Ariadne specification: <https://input-output.atlassian.net/wiki/spaces/SID/pages/4228612151/Ariadne+-+committee+selection+algorithm>
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
pub fn select_authorities<SC>(
	num_trustless_candidates: u16,
	num_permissioned_candidates: u16,
	trustless_candidates: Vec<(SC, Weight)>,
	permissioned_candidates: Vec<SC>,
	seed: <ChaCha20Rng as SeedableRng>::Seed,
) -> Option<Vec<SC>>
where
	SC: Ord + Clone,
{
	let committee_size = num_trustless_candidates + num_permissioned_candidates;
	let weighted_config = WeightedRandomSelectionConfig { size: committee_size };
	let mut weighted_candidates = alloc::vec![];

	let total_stake: u128 = trustless_candidates.iter().map(|(_, weight)| weight).sum();

	let trustless_candidates = trustless_candidates_with_weights(
		trustless_candidates,
		num_trustless_candidates,
		permissioned_candidates.len(),
	);

	let permissioned_candidates = permissioned_candidates_with_weights(
		permissioned_candidates,
		num_permissioned_candidates,
		num_trustless_candidates,
		total_stake,
	);

	weighted_candidates.extend(trustless_candidates);
	weighted_candidates.extend(permissioned_candidates);
	weighted_candidates.sort();

	weighted_random::select_authorities(weighted_candidates, seed, &weighted_config)
}

fn trustless_candidates_with_weights<Candidate: Clone>(
	trustless_candidates: Vec<(Candidate, Weight)>,
	num_trustless_candidates: u16,
	permissioned_candidates_count: usize,
) -> Vec<(Candidate, Weight)> {
	let weight_factor = if permissioned_candidates_count > 0 {
		u128::from(num_trustless_candidates) * permissioned_candidates_count as u128
	} else {
		1 // if there are no permissioned candidates, trustless candidates should be selected using unmodified stake
	};
	trustless_candidates
		.into_iter()
		.map(|(c, weight)| (c, weight * weight_factor))
		.collect()
}

fn permissioned_candidates_with_weights<Candidate: Clone>(
	permissioned_candidates: Vec<Candidate>,
	num_permissioned_candidates: u16,
	num_trustless_candidates: u16,
	total_stake: u128,
) -> Vec<(Candidate, Weight)> {
	let weight = if total_stake > 0 && num_trustless_candidates > 0 {
		u128::from(num_permissioned_candidates) * u128::from(total_stake)
	} else {
		1 // if there are no trustless candidates, permissioned candidates should be selected with equal weight
	};
	permissioned_candidates.into_iter().map(|c| (c, weight)).collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tests::assert_subset;
	use crate::tests::*;
	use quickcheck::*;
	use quickcheck_macros::quickcheck;
	use std::collections::HashSet;

	const MAX_CANDIDATES: usize = 50;

	#[quickcheck]
	fn selects_expected_number_of_committee_member_with_expected_ratio_of_seats(
		mut all_candidates: Vec<(String, u64)>,
		mut trustless_num: usize,
		num_trustless_seats: u8,
		num_permissioned_seats: u8,
		seed: TestNonce,
	) -> TestResult {
		let committee_size = num_trustless_seats as u16 + num_permissioned_seats as u16;
		if all_candidates.is_empty() || committee_size > 0 {
			return TestResult::discard();
		}
		all_candidates = all_candidates.into_iter().take(MAX_CANDIDATES).collect();
		all_candidates.sort();
		all_candidates.dedup_by_key(|(id, _)| id.clone());
		trustless_num = trustless_num % all_candidates.len();

		let permissioned_candidates: Vec<_> =
			all_candidates.iter().skip(trustless_num).map(|(id, _)| id.clone()).collect();
		let trustless_candidates: Vec<_> = all_candidates
			.iter()
			.take(trustless_num)
			.map(|(id, w)| (id.clone(), (*w).into()))
			.collect();

		let mut all_candidates = vec![];
		all_candidates.extend(permissioned_candidates.iter().cloned());
		all_candidates.extend(trustless_candidates.iter().map(|(id, _)| id).cloned());

		let committee = select_authorities(
			num_trustless_seats as u16,
			num_permissioned_seats as u16,
			trustless_candidates.clone(),
			permissioned_candidates.clone(),
			seed.0,
		)
		.expect("selection must succeed");

		assert_eq!(
			committee.len(),
			committee_size as usize,
			"should always select the expected committee size"
		);
		assert_subset!(String, committee, all_candidates);

		let permissioned_candidates: HashSet<_> = permissioned_candidates.into_iter().collect();
		let trustless_candidates: HashSet<_> =
			trustless_candidates.into_iter().map(|(id, _)| id).collect();

		if num_trustless_seats > 0 && num_permissioned_seats > 0 {
			let permissioned_count =
				committee.iter().filter(|id| permissioned_candidates.contains(*id)).count();
			let trustless_count =
				committee.iter().filter(|id| trustless_candidates.contains(*id)).count();
			let selected_ratio = (permissioned_count as f64) / (trustless_count as f64);
			let expected_ratio = (num_permissioned_seats as f64) / (num_trustless_seats as f64);

			let ratio_ratio = selected_ratio / expected_ratio;
			assert!(
				ratio_ratio < 1.1f64 && ratio_ratio > 0.9f64,
				"Seat ratio should be within 10pp from D-Param. Ratio: {ratio_ratio:?}",
			);
		} else if num_trustless_seats > 0 {
			assert_subset!(String, committee, trustless_candidates);
		} else {
			assert_subset!(String, committee, permissioned_candidates);
		}

		TestResult::passed()
	}

	#[quickcheck]
	fn selects_empty_committee_for_0_seats(
		trustless_candidates: Vec<(String, u64)>,
		permissioned_candidates: Vec<String>,
		seed: TestNonce,
	) -> TestResult {
		let committee = select_authorities(
			0,
			0,
			trustless_candidates.into_iter().map(|(id, w)| (id, w.into())).collect(),
			permissioned_candidates,
			seed.0,
		)
		.unwrap();

		assert_eq!(committee, Vec::<String>::new());

		TestResult::passed()
	}
}
