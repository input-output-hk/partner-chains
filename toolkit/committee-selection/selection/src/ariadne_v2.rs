use crate::{Weight, WeightedRandomSelectionConfig};
use alloc::vec::Vec;
use core::iter::repeat_n;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

/// Selects committee according to D-parameter and candidates lists.
/// Resulting committee has size of `registered_seats + permissioned_seats`.
/// If both `registered_candidates` and `permissioned_candidates` are not empty then the
/// selected committee has exactly `registered_seats` assigned to `registered_candidates`.
///
/// Let `E_i` be the expected number of places in the resulting committee of candidate `i`
/// calculated from D-parameter and its weight relative to other candidates.
/// Then candidate `i` is guaranteed to get at least `floor[E_i]` seats.
///
/// Edge cases:
/// If candidates of one type are missing, then their seats are assigned to candidates of other
/// type. It is because D-parameter is desired not mandatory ratio.
/// If `registered_seats` and `permissioned_seats` are 0, empty committee is returned.
/// It is same as for original Ariadne.
pub fn select_authorities<SC>(
	registered_seats: u16,
	permissioned_seats: u16,
	registered_candidates: Vec<(SC, Weight)>,
	permissioned_candidates: Vec<SC>,
	seed: <ChaCha20Rng as SeedableRng>::Seed,
) -> Option<Vec<SC>>
where
	SC: Ord + Clone,
{
	let seats_total = permissioned_seats + registered_seats;
	let mut rng = ChaCha20Rng::from_seed(seed);
	let permissioned_candidates: Vec<(SC, Weight)> =
		permissioned_candidates.into_iter().map(|c| (c, 1)).collect();

	let mut selected = if !registered_candidates.is_empty() && !permissioned_candidates.is_empty() {
		let registered_selected =
			weighted_with_guaranteed_assignment(&registered_candidates, registered_seats, &mut rng);
		let permissioned_selected = weighted_with_guaranteed_assignment(
			&permissioned_candidates,
			permissioned_seats,
			&mut rng,
		);
		let mut selected = registered_selected;
		selected.extend(permissioned_selected);
		selected
	} else if registered_candidates.is_empty() {
		// in absence of registered candidates try to fill their seats with permissioned
		weighted_with_guaranteed_assignment(&permissioned_candidates, seats_total, &mut rng)
	} else {
		// in absence of permissioned candidate try to fill their seats with registered
		weighted_with_guaranteed_assignment(&registered_candidates, seats_total, &mut rng)
	};
	selected.shuffle(&mut rng);
	if selected.is_empty() && seats_total > 0 {
		None
	} else {
		Some(selected)
	}
}

struct SelectGuaranteedResult<T> {
	selected: Vec<T>,
	remaining: Vec<(T, Weight)>,
}

/// Pseudo-random selection with repetitions, but for any candidate that has expected number
/// of seats as P+Q, where P is non-negative integer and Q is in [0, 1), it guarantees at least
/// P places.
pub fn weighted_with_guaranteed_assignment<T: Clone + Ord>(
	candidates: &[(T, Weight)],
	n: u16,
	rng: &mut ChaCha20Rng,
) -> Vec<T> {
	if candidates.is_empty() || n == 0 {
		return Vec::with_capacity(0);
	}
	let SelectGuaranteedResult { mut selected, remaining } = select_guaranteed(candidates, n);
	let selected_count: u16 = selected.len().try_into().expect("selected count can exceed u16");
	selected.extend(select_remaining(remaining, n - selected_count, rng));
	selected
}

fn select_guaranteed<T: Clone + Ord>(
	weighted_candidates: &[(T, Weight)],
	n: u16,
) -> SelectGuaranteedResult<T> {
	let threshold: u128 = weighted_candidates.iter().map(|(_, weight)| weight).sum();
	let scale: u128 = u128::from(n);
	let scaled_candidates = weighted_candidates
		.iter()
		.filter(|(_, weight)| *weight > 0)
		.map(|(c, weight)| (c.clone(), weight * scale));
	let mut selected = Vec::with_capacity(n.into());
	let mut remaining = Vec::with_capacity(n.into());
	for (c, weight) in scaled_candidates {
		let guaranteed: usize = (weight / threshold).try_into().expect("fits u16");
		let remaining_weight = weight % threshold;
		selected.extend(repeat_n(c.clone(), guaranteed));
		if remaining_weight > 0 {
			remaining.push((c.clone(), remaining_weight))
		}
	}
	SelectGuaranteedResult { selected, remaining }
}

fn select_remaining<T: Clone>(
	weighted_candidates: Vec<(T, Weight)>,
	n: u16,
	rng: &mut ChaCha20Rng,
) -> Vec<T> {
	crate::weighted_random::select_authorities(
		weighted_candidates,
		rng.get_seed(),
		&WeightedRandomSelectionConfig { size: n },
	)
	.unwrap_or_else(|| Vec::with_capacity(0))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tests::*;
	use quickcheck_macros::quickcheck;

	#[quickcheck]
	fn guaranteed_places_are_given_no_empty(seed: TestNonce) {
		let big_stake = 9223372036854775807u128;
		// P=3, so in each case P1 and P2 get 1 seat guaranteed. 1 is left for random selection.
		// R1 has twice stake of R2 and R3, so it gets 2 seats, R2 and R3 1 seat. 1 is left for random selection.
		let committee = select_authorities(
			5,
			3,
			vec![("R1", big_stake * 2), ("R2", big_stake), ("R3", big_stake)],
			vec!["P1", "P2"],
			seed.0,
		)
		.unwrap();
		let p1 = committee.iter().filter(|c| **c == "P1").count();
		let r1 = committee.iter().filter(|c| **c == "R1").count();
		let r2 = committee.iter().filter(|c| **c == "R2").count();
		assert!(1 <= p1 && p1 <= 2);
		assert!(2 <= r1 && r1 <= 3);
		assert!(1 <= r2 && r2 <= 2);
		assert_eq!(committee.len(), 8);
	}

	#[quickcheck]
	fn permissioned_get_p_seats_registered_get_r_seats(p: u8, r: u8, seed: TestNonce) {
		// There are more candidates of given type than places for them.
		// No candidate has guaranteed place, only P:R ratio is guaranteed

		let p: u16 = p.into();
		let r: u16 = r.into();
		let registered_candidates: Vec<_> =
			(0..2 * r).into_iter().map(|i| (format!("R{i}"), 1)).collect();
		let permissioned_candidates: Vec<_> =
			(0..2 * r).into_iter().map(|i| format!("P{i}")).collect();
		let committee = select_authorities(
			r.into(),
			p.into(),
			registered_candidates,
			permissioned_candidates,
			seed.0,
		)
		.unwrap();
		let permissioned = committee.iter().filter(|c| c.starts_with("P")).count();
		let registered = committee.iter().filter(|c| c.starts_with("R")).count();
		assert_eq!(permissioned, 3);
		assert_eq!(registered, 2);
	}

	#[quickcheck]
	fn guaranteed_places_are_given_only_registered(seed: TestNonce) {
		// committee size is 11, each candidate has 3 guaranteed places,
		// and there are 2 places for random selection.
		let committee =
			select_authorities(5, 6, vec![("R1", 100), ("R2", 100), ("R3", 100)], vec![], seed.0)
				.unwrap();
		for key in vec!["R1", "R2", "R3"] {
			let candidate_count = committee.iter().filter(|c| **c == key).count();
			assert!(3 <= candidate_count && candidate_count <= 5)
		}
	}

	#[quickcheck]
	fn guaranteed_places_are_given_only_permissioned(seed: TestNonce) {
		let permissioned = vec!["P1", "P2", "P3"];
		// committee size is 10, so each has 3 guaranteed places,
		// and there is 1 place for random selection
		let committee = select_authorities(5, 5, vec![], permissioned, seed.0).unwrap();
		for key in vec!["P1", "P2", "P3"] {
			let candidate_count = committee.iter().filter(|c| **c == key).count();
			assert!(3 <= candidate_count && candidate_count <= 4)
		}
	}

	#[test]
	fn remaining_seats_are_given_according_to_weights() {
		// Each permissioned is expected to have 4/3 = 1.333 seats.
		let permissioned = vec!["P1", "P2", "P3"];
		// R1 is expected to have 0.4*3=1.2, R2 is expected to have 0.6*3=1.8 seats.
		let registered = vec![("R1", 40), ("R2", 60)];

		let mut p1_count = 0;
		let mut r1_count = 0;
		let mut r2_count = 0;
		for i in 0u32..1000 {
			let mut seed = [0u8; 32];
			seed[0..4].copy_from_slice(&i.to_le_bytes());

			let committee =
				select_authorities(3, 4, registered.clone(), permissioned.clone(), seed).unwrap();
			p1_count += committee.iter().filter(|c| **c == "P1").count();
			r1_count += committee.iter().filter(|c| **c == "R1").count();
			r2_count += committee.iter().filter(|c| **c == "R2").count();
		}
		let tolerance = 5;
		assert!(1333 - tolerance <= p1_count && p1_count <= 1333 + tolerance);
		assert!(1200 - tolerance <= r1_count && r1_count <= 1200 + tolerance);
		assert!(1800 - tolerance <= r2_count && r2_count <= 1800 + tolerance);
	}

	#[test]
	fn use_registered_candidates_when_r_is_0_and_there_are_no_permissioned_candidates() {
		let committee =
			select_authorities(0, 3, vec![("R1", 100), ("R2", 100)], vec![], [0u8; 32]).unwrap();
		assert_eq!(committee.len(), 3)
	}

	#[test]
	fn use_permissioned_candidates_when_p_is_0_and_there_are_no_registered_candidates() {
		let committee = select_authorities(3, 0, vec![], vec!["P1", "P2"], [0u8; 32]).unwrap();
		assert_eq!(committee.len(), 3)
	}

	#[quickcheck]
	fn selects_empty_committee_for_0_seats(
		trustless_candidates: Vec<(String, u128)>,
		permissioned_candidates: Vec<String>,
		seed: TestNonce,
	) {
		let committee =
			select_authorities(0, 0, trustless_candidates, permissioned_candidates, seed.0)
				.unwrap();
		assert_eq!(committee, Vec::<String>::new());
	}
}
