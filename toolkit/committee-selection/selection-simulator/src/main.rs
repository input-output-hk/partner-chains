use clap::*;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::SeedableRng;
use serde::*;

mod inherent_risk;
mod simple_sim;

#[derive(clap::Parser, Debug)]
pub enum TopCommand {
	SimpleSim(simple_sim::Command),
	InherentRisk(inherent_risk::Command),
}

#[derive(Serialize, Deserialize, Debug)]
struct SPO {
	key: String,
	stake: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct Registered {
	key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, clap::ValueEnum)]
enum AriadneVersion {
	V1,
	V2,
}

impl AriadneVersion {
	pub fn select_authorities<SC>(
		&self,
		registered_seats: u16,
		permissioned_seats: u16,
		registered_candidates: Vec<(SC, selection::Weight)>,
		permissioned_candidates: Vec<SC>,
		seed: <ChaCha20Rng as SeedableRng>::Seed,
	) -> Option<Vec<SC>>
	where
		SC: Ord + Clone,
	{
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
	serde_json::from_reader::<_, Vec<SPO>>(
		std::fs::File::open(file).expect("Registered candidates file can't be opened"),
	)
	.expect("Registered candidates file is invalid")
	.into_iter()
	.map(|spo| (spo.key, spo.stake.into()))
	.collect()
}

fn load_permissioned(file: String) -> Vec<String> {
	serde_json::from_reader::<_, Vec<Registered>>(
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
		TopCommand::SimpleSim(cmd) => cmd.execute(rng),
		TopCommand::InherentRisk(cmd) => cmd.execute(rng),
	}
}
