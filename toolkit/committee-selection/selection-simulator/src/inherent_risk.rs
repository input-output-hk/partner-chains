use crate::*;
use itertools::*;
use rand_chacha::ChaCha20Rng;

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
	permissioned_pool_size: u32,
	#[arg(long, default_value = "v2")]
	ariadne_version: AriadneVersion,
}

impl Command {
	pub fn execute(self, mut rng: ChaCha20Rng) {
		let potential_registered_candidates: Vec<(String, u128)> =
			self.registered_file.map(load_registered).unwrap_or_default();

		let permissioned_candidates: Vec<String> =
			self.permissioned_file.map(load_permissioned).unwrap_or_default();

		log::info!(
			"Number of potential registered candidates: {}",
			potential_registered_candidates.len()
		);
		log::info!("Number of permissioned candidates: {}", permissioned_candidates.len());

		println!("safe_offline_members,distinct_members,max_single_member_seats");
		for i in 0..self.repetitions {
			if i % 100 == 0 && i > 0 {
				log::info!("Generation progress: {i}/{}", self.repetitions);
			}
			let mut seed = [0u8; 32];
			rng.fill(&mut seed);

			// sample a registered candidate pool from all existing SPOs
			let mut registered_candidates = potential_registered_candidates
				.clone()
				.into_iter()
				.choose_multiple(&mut rng, self.permissioned_pool_size as usize);
			registered_candidates.shuffle(&mut rng);

			let committee = self
				.ariadne_version
				.select_authorities(
					self.registered_seats.clone(),
					self.permissioned_seats.clone(),
					registered_candidates,
					permissioned_candidates.clone(),
					seed,
				)
				.expect("Selection failed");

			let mut member_seat_counts: Vec<_> = (committee.clone().iter())
				.into_group_map_by(|&v| v)
				.into_iter()
				.map(|(_, vs)| vs.len() as u16)
				.collect();
			member_seat_counts.sort();
			member_seat_counts.reverse();
			log::debug!("Powers: {member_seat_counts:?}");

			let total_seats = self.permissioned_seats + self.registered_seats;
			assert_eq!(total_seats, member_seat_counts.iter().sum::<u16>());

			// find the number of top seat members that can safely go offline
			let mut safe_offline_members = 0;
			let safety_threshold = (total_seats - 1) / 3;
			let mut seats = 0;
			for power in &member_seat_counts {
				seats += power;
				if seats <= safety_threshold { safe_offline_members += 1 } else { break }
			}

			println!(
				"{safe_offline_members},{},{}",
				member_seat_counts.len(),
				member_seat_counts[0]
			);
		}
	}
}
