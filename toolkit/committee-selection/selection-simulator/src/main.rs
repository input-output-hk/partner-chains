//! Executable crate for simulating committee selection with the Ariadne algorithm
#![deny(missing_docs)]

use clap::*;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::SeedableRng;
use serde::*;
use std::fmt::Display;

mod analyze;
mod simple_sim;

/// Top level command of the executable. Subcommands are various ways of simulating Ariadne selection.
#[derive(clap::Parser, Debug)]
pub enum TopCommand {
	/// Simulates Ariadne selection and prints selected committees as JSON
	Simulate(simple_sim::Command),
	/// Simulates Ariadne selection and prints various statistics about the selected committees as CSV
	Analyze(analyze::Command),
}

#[derive(Serialize, Deserialize, Debug)]
struct SPO {
	key: String,
	stake: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct Permissioned {
	key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, clap::ValueEnum)]
enum AriadneVersion {
	V1,
	V2,
}

impl Display for AriadneVersion {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(match self {
			Self::V1 => "v1",
			Self::V2 => "v2",
		})
	}
}

impl AriadneVersion {
	pub fn select_authorities<SC>(
		&self,
		registered_seats: u16,
		permissioned_seats: u16,
		registered_candidates: Vec<(SC, selection::Weight)>,
		permissioned_candidates: Vec<SC>,
		rng: &mut ChaCha20Rng,
	) -> Option<Vec<SC>>
	where
		SC: Ord + Clone,
	{
		let mut seed = [0u8; 32];
		rng.fill(&mut seed);
		match self {
			Self::V1 => selection::ariadne::select_authorities(
				registered_seats,
				permissioned_seats,
				registered_candidates,
				permissioned_candidates,
				seed,
			),
			Self::V2 => selection::ariadne_v2::select_authorities(
				registered_seats,
				permissioned_seats,
				registered_candidates,
				permissioned_candidates,
				seed,
			),
		}
	}
}

fn load_registered(file: String) -> Vec<(String, u128)> {
	let file = std::fs::File::open(file).expect("Registered candidates file can't be opened");
	serde_json::from_reader::<_, Vec<SPO>>(file)
		.expect("Registered candidates file is invalid")
		.into_iter()
		.map(|spo| (spo.key, spo.stake.into()))
		.collect()
}

fn load_permissioned(file: String) -> Vec<String> {
	serde_json::from_reader::<_, Vec<Permissioned>>(
		std::fs::File::open(file).expect("Permissioned candidates file can't be opened"),
	)
	.expect("Permissioned candidates file is invalid")
	.into_iter()
	.map(|p| p.key)
	.collect()
}

fn main() {
	env_logger::builder().filter_level(log::LevelFilter::Info).init();

	let cmd = TopCommand::parse();

	let rng = ChaCha20Rng::from_os_rng();

	match cmd {
		TopCommand::Simulate(cmd) => cmd.execute(rng),
		TopCommand::Analyze(cmd) => cmd.execute(rng),
	}
}
