use crate::*;
use rand_chacha::ChaCha20Rng;

/// Runs Ariadne selection and prints the selected committee as JSON
#[derive(clap::Parser, Debug)]
pub struct Command {
	/// Number of permissioned seats
	#[arg(long, short = 'P')]
	permissioned_seats: u16,
	/// Number of registered seats
	#[arg(long, short = 'R')]
	registered_seats: u16,
	/// File containing permissioned candidates, defaults to no permissioned candidates
	#[arg(long, short = 'p')]
	permissioned_file: Option<String>,
	/// File containing registered candidates, defaults to no registered candidates
	#[arg(long, short = 'r')]
	registered_file: Option<String>,
	/// Number of committees to select. Each committee will be a separate JSON array
	#[arg(long, default_value = "1")]
	repetitions: u32,
	/// Ariadne algorithm version
	#[arg(long, default_value = "v2")]
	ariadne_version: AriadneVersion,
}

impl Command {
	/// Executes the command using givern RNG
	pub fn execute(self, mut rng: ChaCha20Rng) {
		let registered_candidates: Vec<(String, u128)> =
			self.registered_file.map(load_registered).unwrap_or_default();

		let permissioned_candidates: Vec<String> =
			self.permissioned_file.map(load_permissioned).unwrap_or_default();

		log::info!("Number of registered candidates: {}", registered_candidates.len());
		log::info!("Number of permissioned candidates: {}", permissioned_candidates.len());

		for i in 0..self.repetitions {
			if i % 100 == 0 && i > 0 {
				log::info!("Generation progress: {i}/{}", self.repetitions);
			}

			let selected = self
				.ariadne_version
				.select_authorities(
					self.registered_seats.clone(),
					self.permissioned_seats.clone(),
					registered_candidates.clone(),
					permissioned_candidates.clone(),
					&mut rng,
				)
				.expect("Selection failed");
			println!("{}", serde_json::to_string(&selected).unwrap());
		}
	}
}
