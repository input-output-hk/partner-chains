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

/// Runs Ariadne selection and calculates various statistics for the selected committee.
///
/// This command writes output data as CSV either to standard output or a file with name
/// `<target-dir>/ariadne-simulation-<timestamp>.csv`.
///
/// The following statistics are calculated for each committee:
/// - `total_registered_stake`: total stake of the registered candidate pool
/// - `total_committee_stake`: total stake of all selected committee members
/// - `distinct_members`: number of unique members in the committee
/// - `max_single_member_seats`: highest number of committee seats occupied by the same member
/// - `safe_offline_members`: highest number of members that can be offline without affecting the consensus.
///                           This number is calculated by greedily taking members with highest stake until
///                           more than 33% of seats are offline.
/// - `top_safe_offline_stake`: total stake of top stake candidates that can be offline without affecting the consensus
/// - `bottom_safe_offline_stake`: total stake of lowest stake candidates that can be offline without affecting the consensus
///
/// Additionally, all input parameters are saved with the data.
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
	/// Number of committees to select. Each committee will have a separate row in the output CSV file
	#[arg(long, default_value = "l")]
	repetitions: u32,
	/// Number of registered candidates to be sampled from the `registered_file`. Defaults to the size of `registered_file`
	#[arg(long)]
	registered_pool_size: Option<u32>,
	/// Ariadne algorithm version
	#[arg(long, default_value = "v2")]
	ariadne_version: AriadneVersion,
	/// Determines whether to output to standard output instead of a file
	#[arg(long, default_value = "false")]
	output_to_terminal: bool,
	/// Directory in which to save the output CSV file
	#[arg(long)]
	target_dir: Option<String>,
}

impl Command {
	/// Executes the command using givern RNG
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
				"{}/ariadne-simulation-{}.csv",
				self.target_dir.clone().unwrap_or(".".into()),
				SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
			);
			&mut BufWriter::new(std::fs::File::create(file_name).unwrap())
		};

		writeln!(
			output,
			"{}",
			[
				"ariadne_version",
				"R",
				"P",
				"registered_candidates",
				"total_registered_stake",
				"registered_file",
				"permissioned_file",
				"total_committee_stake",
				"distinct_members",
				"max_single_member_seats",
				"safe_offline_members",
				"top_safe_offline_stake",
				"bottom_safe_offline_stake"
			]
			.join(",")
		)
		.unwrap();
		for i in 0..self.repetitions {
			if i % 100 == 0 && i > 0 {
				log::info!("Generation progress: {i}/{}", self.repetitions);
			}

			// sample a registered candidate pool from all existing SPOs
			let (registered_pool_size, registered_candidates) =
				self.sample_registered(potential_registered_candidates.clone(), &mut rng);

			let committee = self
				.ariadne_version
				.select_authorities(
					self.registered_seats,
					self.permissioned_seats,
					registered_candidates.clone(),
					permissioned_candidates.clone(),
					&mut rng,
				)
				.expect("Selection failed");

			let SelectionStats {
				bottom_safe_offline_stake,
				top_safe_offline_stake,
				distinct_members,
				max_single_member_seats,
				safe_offline_members,
				total_committee_stake,
				total_registered_stake,
			} = self.calculate_stats(&committee, &registered_candidates);

			let registered_file =
				self.registered_file.clone().map_or("null".to_string(), |f| format!("{f:?}"));
			let permissioned_file =
				self.permissioned_file.clone().map_or("null".to_string(), |f| format!("{f:?}"));

			writeln!(
				output,
				"{},{},{},{},{},{},{},{},{},{},{},{},{}",
				self.ariadne_version,
				self.registered_seats,
				self.permissioned_seats,
				registered_pool_size,
				total_registered_stake,
				registered_file,
				permissioned_file,
				total_committee_stake,
				distinct_members,
				max_single_member_seats,
				safe_offline_members,
				top_safe_offline_stake,
				bottom_safe_offline_stake
			)
			.expect("Failed to write CSV data row");
		}
	}

	fn sample_registered(
		&self,
		potential_registered_candidates: Vec<(String, u128)>,
		rng: &mut ChaCha20Rng,
	) -> (usize, Vec<(String, u128)>) {
		let (registered_pool_size, mut registered_candidates) = match self.registered_pool_size {
			None => {
				(potential_registered_candidates.len(), potential_registered_candidates.clone())
			},
			Some(registered_pool_size) => {
				let registered_pool_size =
					(registered_pool_size as usize).min(potential_registered_candidates.len());
				let candidates = potential_registered_candidates
					.clone()
					.into_iter()
					.choose_multiple(rng, registered_pool_size);
				(registered_pool_size, candidates)
			},
		};
		registered_candidates.shuffle(rng);
		(registered_pool_size, registered_candidates)
	}

	fn calculate_stats(
		&self,
		committee: &[String],
		registered_candidates: &[(String, u128)],
	) -> SelectionStats {
		let stake_lookup: HashMap<String, u128> = registered_candidates.iter().cloned().collect();
		let mut member_seat_counts: Vec<(u16, String, u128)> = (committee.iter().cloned())
			.into_group_map_by(|v| v.clone())
			.into_iter()
			.map(|(id, vs)| {
				(vs.len() as u16, id.clone(), stake_lookup.get(&id).cloned().unwrap_or_default())
			})
			.collect();
		member_seat_counts.sort();
		member_seat_counts.reverse();
		let total_seats = self.permissioned_seats + self.registered_seats;

		// find the number of top seat members that can safely go offline
		let mut safe_offline_members = 0;
		let safety_threshold = (total_seats - 1) / 3;
		let mut seats = 0;
		let mut total_committee_stake = 0;
		let mut top_safe_offline_stake = 0;
		for (power, _, stake) in &member_seat_counts {
			seats += power;
			total_committee_stake += stake;
			if seats <= safety_threshold {
				top_safe_offline_stake += stake;
				safe_offline_members += 1
			}
		}

		let mut seats = 0;
		let mut bottom_safe_offline_stake = 0;
		for (power, _id, stake) in member_seat_counts.iter().rev() {
			seats += power;
			if seats <= safety_threshold {
				bottom_safe_offline_stake += stake;
			}
		}

		let total_registered_stake: u128 = registered_candidates.iter().map(|c| c.1).sum();
		SelectionStats {
			bottom_safe_offline_stake,
			top_safe_offline_stake,
			distinct_members: member_seat_counts.len(),
			max_single_member_seats: member_seat_counts[0].0,
			safe_offline_members: safe_offline_members as usize,
			total_committee_stake,
			total_registered_stake,
		}
	}
}

struct SelectionStats {
	bottom_safe_offline_stake: u128,
	top_safe_offline_stake: u128,
	distinct_members: usize,
	max_single_member_seats: u16,
	safe_offline_members: usize,
	total_committee_stake: u128,
	total_registered_stake: u128,
}
