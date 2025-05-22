use crate::*;
use rand_chacha::ChaCha20Rng;
use selection::ariadne_v2::select_authorities;

#[derive(clap::Parser, Debug)]
pub struct Command {
	#[arg(long, short = 'P')]
	permissioned_seats: u16,
	#[arg(long, short = 'R')]
	registered_seats: u16,
	#[arg(long, short = 'p')]
	permissioned_file: Option<String>,
	#[arg(long, short = 'r')]
	registered_file: Option<String>,
	#[arg(long, default_value = "1")]
	repetitions: u32,
}

impl Command {
	pub fn execute(self, mut rng: ChaCha20Rng) {
		let registered_candidates: Vec<(String, u128)> =
			self.registered_file.map(load_registered).unwrap_or_default();

		let permissioned_candidates: Vec<String> =
			self.permissioned_file.map(load_permissioned).unwrap_or_default();

		log::info!("Number of registered candidates: {}", registered_candidates.len());
		log::info!("Number of permissioned candidates: {}", permissioned_candidates.len());

		for _ in 0..self.repetitions {
			let mut seed = [0u8; 32];
			rng.fill(&mut seed);
			let selected = select_authorities(
				self.registered_seats.clone(),
				self.permissioned_seats.clone(),
				registered_candidates.clone(),
				permissioned_candidates.clone(),
				seed,
			)
			.expect("Selection failed");
			println!("{}", serde_json::to_string(&selected).unwrap());
		}
	}
}
