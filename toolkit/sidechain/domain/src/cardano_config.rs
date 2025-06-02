//! Unified Cardano configuration module that consolidates all Cardano-related parameters

use crate::mainchain_epoch::MainchainEpochConfig;
#[cfg(feature = "std")]
use figment::{
	Figment,
	providers::{Env, Serialized},
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Unified configuration for all Cardano-related parameters used across the Partner Chains toolkit
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct CardanoConfig {
	/// Mainchain epoch configuration (timing, slots, epochs)
	#[cfg_attr(feature = "std", serde(flatten))]
	pub epoch_config: MainchainEpochConfig,

	/// Cardano security parameter `k`. Number of blocks after which the chain is considered final on mainchain.
	pub cardano_security_parameter: u32,

	/// Cardano active slots coefficient `f`. Probability of a slot being occupied.
	pub cardano_active_slots_coeff: f32,
}

impl Default for CardanoConfig {
	fn default() -> Self {
		Self {
			epoch_config: MainchainEpochConfig {
				epoch_duration_millis: sp_core::offchain::Duration::from_millis(432000000), // 5 days
				slot_duration_millis: sp_core::offchain::Duration::from_millis(1000),       // 1 second
				first_epoch_timestamp_millis: sp_core::offchain::Timestamp::from_unix_millis(
					1596059091000,
				), // Shelley start on mainnet
				first_epoch_number: 208,
				first_slot_number: 4492800,
			},
			cardano_security_parameter: 432,
			cardano_active_slots_coeff: 0.05,
		}
	}
}

#[cfg(feature = "std")]
impl CardanoConfig {
	/// Reads the unified Cardano configuration from environment variables
	///
	/// Environment variables:
	/// - `MC__EPOCH_DURATION_MILLIS`
	/// - `MC__SLOT_DURATION_MILLIS`
	/// - `MC__FIRST_EPOCH_TIMESTAMP_MILLIS`
	/// - `MC__FIRST_EPOCH_NUMBER`
	/// - `MC__FIRST_SLOT_NUMBER`
	/// - `CARDANO_SECURITY_PARAMETER`
	/// - `CARDANO_ACTIVE_SLOTS_COEFF`
	pub fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		// First read the epoch config
		let epoch_config = MainchainEpochConfig::read_from_env()?;

		// Then read the additional Cardano parameters
		#[derive(Serialize, Deserialize)]
		struct CardanoParams {
			#[serde(default = "default_security_parameter")]
			cardano_security_parameter: u32,
			#[serde(default = "default_active_slots_coeff")]
			cardano_active_slots_coeff: f32,
		}

		fn default_security_parameter() -> u32 {
			432
		}
		fn default_active_slots_coeff() -> f32 {
			0.05
		}

		let params: CardanoParams = Figment::new()
			.merge(Serialized::defaults(CardanoParams {
				cardano_security_parameter: default_security_parameter(),
				cardano_active_slots_coeff: default_active_slots_coeff(),
			}))
			.merge(Env::raw())
			.extract()?;

		Ok(Self {
			epoch_config,
			cardano_security_parameter: params.cardano_security_parameter,
			cardano_active_slots_coeff: params.cardano_active_slots_coeff,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_core::offchain::{Duration, Timestamp};

	#[test]
	fn read_cardano_config_from_env() {
		figment::Jail::expect_with(|jail| {
			// Set epoch config env vars
			jail.set_env("MC__FIRST_EPOCH_TIMESTAMP_MILLIS", 10);
			jail.set_env("MC__FIRST_EPOCH_NUMBER", 100);
			jail.set_env("MC__EPOCH_DURATION_MILLIS", 1000);
			jail.set_env("MC__FIRST_SLOT_NUMBER", 42);
			jail.set_env("MC__SLOT_DURATION_MILLIS", 500);

			// Set Cardano-specific env vars
			jail.set_env("CARDANO_SECURITY_PARAMETER", 200);
			jail.set_env("CARDANO_ACTIVE_SLOTS_COEFF", 0.1);

			let config = CardanoConfig::from_env().expect("Should succeed");

			assert_eq!(
				config.epoch_config.first_epoch_timestamp_millis,
				Timestamp::from_unix_millis(10)
			);
			assert_eq!(config.epoch_config.first_epoch_number, 100);
			assert_eq!(config.epoch_config.epoch_duration_millis, Duration::from_millis(1000));
			assert_eq!(config.epoch_config.first_slot_number, 42);
			assert_eq!(config.epoch_config.slot_duration_millis, Duration::from_millis(500));
			assert_eq!(config.cardano_security_parameter, 200);
			assert_eq!(config.cardano_active_slots_coeff, 0.1);

			Ok(())
		});
	}

	#[test]
	fn read_cardano_config_with_defaults() {
		figment::Jail::expect_with(|jail| {
			// Set only epoch config env vars
			jail.set_env("MC__FIRST_EPOCH_TIMESTAMP_MILLIS", 10);
			jail.set_env("MC__FIRST_EPOCH_NUMBER", 100);
			jail.set_env("MC__EPOCH_DURATION_MILLIS", 1000);
			jail.set_env("MC__FIRST_SLOT_NUMBER", 42);

			let config = CardanoConfig::from_env().expect("Should succeed");

			// Check that defaults are used for unset values
			assert_eq!(config.cardano_security_parameter, 432);
			assert_eq!(config.cardano_active_slots_coeff, 0.05);

			Ok(())
		});
	}
}
