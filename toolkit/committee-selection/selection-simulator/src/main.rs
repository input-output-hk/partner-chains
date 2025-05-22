use clap::*;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::SeedableRng;
use selection::ariadne_v2::select_authorities;
use serde::*;

#[derive(clap::Parser, Debug)]
struct Command {
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

#[derive(Serialize, Deserialize, Debug)]
struct SPO {
	key: String,
	stake: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct Registered {
	key: String,
}

fn main() {
	env_logger::builder().filter_level(log::LevelFilter::Info).init();

	let cmd = Command::parse();

	let registered_candidates: Vec<(String, u128)> = cmd
		.registered_file
		.map(|file| {
			serde_json::from_reader::<_, Vec<SPO>>(
				std::fs::File::open(file).expect("Registered candidates file can't be opened"),
			)
			.expect("Registered candidates file is invalid")
			.into_iter()
			.map(|spo| (spo.key, spo.stake.into()))
			.collect()
		})
		.unwrap_or_default();

	let permissioned_candidates: Vec<String> = cmd
		.permissioned_file
		.map(|file| {
			serde_json::from_reader::<_, Vec<Registered>>(
				std::fs::File::open(file).expect("Permissioned candidates file can't be opened"),
			)
			.expect("Permissioned candidates file is invalid")
			.into_iter()
			.map(|p| p.key)
			.collect()
		})
		.unwrap_or_default();

	log::info!("Number of registered candidates: {}", registered_candidates.len());
	log::info!("Number of permissioned candidates: {}", permissioned_candidates.len());

	let mut rng = ChaCha20Rng::from_os_rng();

	for _ in 0..cmd.repetitions {
		let mut seed = [0u8; 32];
		rng.fill(&mut seed);
		let selected = select_authorities(
			cmd.registered_seats.clone(),
			cmd.permissioned_seats.clone(),
			registered_candidates.clone(),
			permissioned_candidates.clone(),
			seed,
		)
		.expect("Selection failed");
		println!("{}", serde_json::to_string(&selected).unwrap());
	}
}
