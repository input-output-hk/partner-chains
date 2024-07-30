#![cfg_attr(not(feature = "std"), no_std)]

pub mod mainchain;

pub use mainchain::*;
pub use sp_core::offchain::{Duration, Timestamp};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize))]
pub struct EpochConfig {
	pub mc: MainchainEpochConfig,
}

impl EpochConfig {
	#[cfg(feature = "std")]
	pub fn read() -> figment::error::Result<EpochConfig> {
		use figment::{providers::Env, Figment};
		let epoch_config: EpochConfig = Figment::new().merge(Env::raw().split("__")).extract()?;
		Ok(epoch_config)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::assert_ok;
	use sp_core::offchain::{Duration, Timestamp};

	#[test]
	fn read_epoch_config_from_env() {
		figment::Jail::expect_with(|jail| {
			set_mainchain_env(jail);
			assert_ok!(
				EpochConfig::read(),
				EpochConfig {
					mc: MainchainEpochConfig {
						first_epoch_timestamp_millis: Timestamp::from_unix_millis(10),
						first_epoch_number: 100,
						epoch_duration_millis: Duration::from_millis(1000),
						first_slot_number: 42,
					},
				}
			);
			Ok(())
		});
	}

	fn set_mainchain_env(jail: &mut figment::Jail) {
		jail.set_env("MC__FIRST_EPOCH_TIMESTAMP_MILLIS", 10);
		jail.set_env("MC__FIRST_EPOCH_NUMBER", 100);
		jail.set_env("MC__EPOCH_DURATION_MILLIS", 1000);
		jail.set_env("MC__FIRST_SLOT_NUMBER", 42);
	}
}
