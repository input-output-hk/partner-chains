use std::{
	collections::HashMap,
	fmt::Debug,
	io::{self, BufWriter},
	time::{SystemTime, UNIX_EPOCH},
};

use crate::*;
use itertools::*;
use rand_chacha::ChaCha20Rng;
use std::io::Write;

/// Select random registered candidate pool from all potential registered candidates
/// and simulates Ariadne selection. Calculates how man top members (by stake) can
/// go offline before the selected committee is no longer able to finalize blocks.
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
	#[arg(long, default_value = "l")]
	repetitions: u32,
	#[arg(long)]
	registered_pool_size: u32,
	#[arg(long, default_value = "v2")]
	ariadne_version: AriadneVersion,
	#[arg(long, default_value = "false")]
	output_to_terminal: bool,
}

impl Command {
	pub fn execute(self, mut rng: ChaCha20Rng) {
		let potential_registered_candidates: Vec<(String, u128)> =
			self.registered_file.clone().map(load_registered).unwrap_or_default();

		let permissioned_candidates: Vec<String> =
			self.permissioned_file.clone().map(load_permissioned).unwrap_or_default();

		log::info!(
			"Number of potential registered candidates: {}",
			potential_registered_candidates.len()
		);
		log::info!("Number of permissioned candidates: {}", permissioned_candidates.len());

		let output: &mut (dyn Write) = if self.output_to_terminal {
			&mut io::stdout()
		} else {
			let file_name = format!(
				"ariadne-simulation-{}.csv",
				SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
			);
			&mut BufWriter::new(std::fs::File::create(file_name).unwrap())
		};

		writeln!(
			output,
			"safe_offline_members,distinct_members,max_single_member_seats,ada_to_attack,R,P,registered_file,permissioned_file,registration_number,ariadne_version"
		)
		.unwrap();
		for i in 0..self.repetitions {
			if i % 100 == 0 && i > 0 {
				log::info!("Generation progress: {i}/{}", self.repetitions);
			}
			let mut seed = [0u8; 32];
			rng.fill(&mut seed);

			// sample a registered candidate pool from all existing SPOs
			let mut registered_candidates =
				potential_registered_candidates.clone().into_iter().choose_multiple(
					&mut rng,
					(self.registered_pool_size as usize).min(potential_registered_candidates.len()),
				);
			registered_candidates.shuffle(&mut rng);

			let committee = self
				.ariadne_version
				.select_authorities(
					self.registered_seats.clone(),
					self.permissioned_seats.clone(),
					registered_candidates.clone(),
					permissioned_candidates.clone(),
					seed,
				)
				.expect("Selection failed");

			let stake_lookup: HashMap<String, u128> = registered_candidates.into_iter().collect();

			let mut member_seat_counts: Vec<_> = (committee.into_iter())
				.into_group_map_by(|v| v.clone())
				.into_iter()
				.map(|(id, vs)| (vs.len() as u16, id.clone(), stake_lookup[&id]))
				.collect();
			member_seat_counts.sort();
			member_seat_counts.reverse();
			log::debug!("Powers: {member_seat_counts:?}");

			let total_seats = self.permissioned_seats + self.registered_seats;

			// find the number of top seat members that can safely go offline
			let mut safe_offline_members = 0;
			let safety_threshold = (total_seats - 1) / 3;
			let mut seats = 0;
			let mut lovelace_to_attack = 0;
			for (power, id, stake) in &member_seat_counts {
				seats += power;
				lovelace_to_attack += stake;
				log::debug!("{id} has {stake} stake");
				if seats <= safety_threshold {
					safe_offline_members += 1
				} else {
					break;
				}
			}
			let ada_to_attack = lovelace_to_attack / 1_000_000;

			writeln!(
				output,
				"{safe_offline_members},{},{},{ada_to_attack},{},{},{},{},{},{}",
				member_seat_counts.len(),
				member_seat_counts[0].0,
				self.registered_seats,
				self.permissioned_seats,
				self.registered_file.clone().map_or("null".to_string(), |f| format!("{f:?}")),
				self.permissioned_file.clone().map_or("null".to_string(), |f| format!("{f:?}")),
				self.registered_pool_size,
				self.ariadne_version
			)
			.unwrap();
		}
	}
}
