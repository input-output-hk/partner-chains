extern crate alloc;
use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode};
use rand::Rng;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

/// Parameters needed for weighted-pseudorandom selection algorithm
#[derive(Encode, Decode, scale_info::TypeInfo)]
pub struct WeightedRandomSelectionConfig {
	pub size: u16,
}

pub type Weight = u128;

impl WeightedRandomSelectionConfig {
	pub fn select_authorities<T: Clone>(
		&self,
		weighted_candidates: Vec<(T, Weight)>,
		seed: <ChaCha20Rng as SeedableRng>::Seed,
	) -> Option<Vec<T>> {
		select_authorities(weighted_candidates, seed, self)
	}
}

pub fn select_authorities<T: Clone>(
	weighted_candidates: Vec<(T, Weight)>,
	seed: <ChaCha20Rng as SeedableRng>::Seed,
	config: &WeightedRandomSelectionConfig,
) -> Option<Vec<T>> {
	let size = usize::from(config.size);
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
