use crate::{McEpochNumber, McSlotNumber};
#[cfg(feature = "std")]
use parity_scale_codec::Decode;
use parity_scale_codec::Encode;
pub use sp_core::offchain::{Duration, Timestamp};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize))]
pub struct MainchainEpochConfig {
	/// First epoch of the Cardano Era that the Sidechain bases itself from. Notice that there is a `first_epoch_number` field below - it represents the "first epoch number" for the Sidechain, but not for the Cardano network.
	pub first_epoch_timestamp_millis: Timestamp,
	pub epoch_duration_millis: Duration,
	/// Number of the Cardano Epoch started at `first_epoch_timestamp_millis`
	pub first_epoch_number: u32,
	pub first_slot_number: u64,
	#[cfg_attr(feature = "std", serde(default = "default_slot_duration"))]
	pub slot_duration_millis: Duration,
}

#[cfg(feature = "std")]
fn default_slot_duration() -> Duration {
	Duration::from_millis(1000)
}

#[derive(Encode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error, sp_core::RuntimeDebug))]
pub enum EpochDerivationError {
	#[cfg_attr(feature = "std", error("Timestamp before first Mainchain Epoch"))]
	TimestampTooSmall,
	#[cfg_attr(feature = "std", error("Epoch number exceeds maximal allowed value"))]
	EpochTooBig,
	#[cfg_attr(feature = "std", error("Epoch number is below the allowed value"))]
	EpochTooSmall,
	#[cfg_attr(feature = "std", error("Slot number is below the allowed value"))]
	SlotTooSmall,
}

pub trait MainchainEpochDerivation {
	fn epochs_passed(&self, timestamp: Timestamp) -> Result<u32, EpochDerivationError>;

	fn timestamp_to_mainchain_epoch(
		&self,
		timestamp: Timestamp,
	) -> Result<McEpochNumber, EpochDerivationError>;

	fn timestamp_to_mainchain_slot_number(
		&self,
		timestamp: Timestamp,
	) -> Result<u64, EpochDerivationError>;

	fn mainchain_epoch_to_timestamp(&self, epoch: McEpochNumber) -> Timestamp;

	fn first_slot_of_epoch(
		&self,
		epoch: McEpochNumber,
	) -> Result<McSlotNumber, EpochDerivationError>;

	fn epoch_for_slot(&self, slot: McSlotNumber) -> Result<McEpochNumber, EpochDerivationError>;
}

impl MainchainEpochConfig {
	fn slots_per_epoch(&self) -> u64 {
		self.epoch_duration_millis.millis() / 1000
	}

	#[cfg(feature = "std")]
	pub fn read_from_env() -> figment::error::Result<Self> {
		figment::Figment::new()
			.merge(figment::providers::Env::prefixed("MC__"))
			.extract()
	}
}

impl MainchainEpochDerivation for MainchainEpochConfig {
	fn epochs_passed(&self, timestamp: Timestamp) -> Result<u32, EpochDerivationError> {
		let time_elapsed = timestamp
			.unix_millis()
			.checked_sub(self.first_epoch_timestamp_millis.unix_millis())
			.ok_or(EpochDerivationError::TimestampTooSmall)?;
		let res: u32 = (time_elapsed / self.epoch_duration_millis.millis())
			.try_into()
			.map_err(|_| EpochDerivationError::EpochTooBig)?;
		if res > i32::MAX as u32 { Err(EpochDerivationError::EpochTooBig) } else { Ok(res) }
	}

	fn timestamp_to_mainchain_epoch(
		&self,
		timestamp: Timestamp,
	) -> Result<McEpochNumber, EpochDerivationError> {
		let epochs_passed = self.epochs_passed(timestamp)?;
		Ok(McEpochNumber(self.first_epoch_number.saturating_add(epochs_passed)))
	}

	fn timestamp_to_mainchain_slot_number(
		&self,
		timestamp: Timestamp,
	) -> Result<u64, EpochDerivationError> {
		let time_elapsed = timestamp
			.unix_millis()
			.checked_sub(self.first_epoch_timestamp_millis.unix_millis())
			.ok_or(EpochDerivationError::TimestampTooSmall)?;
		Ok(self.first_slot_number + time_elapsed / self.slot_duration_millis.millis())
	}

	fn mainchain_epoch_to_timestamp(&self, epoch: McEpochNumber) -> Timestamp {
		let time_elapsed = self.epoch_duration_millis.millis() * epoch.0 as u64;
		Timestamp::from_unix_millis(self.first_epoch_timestamp_millis.unix_millis() + time_elapsed)
	}

	fn first_slot_of_epoch(
		&self,
		epoch: McEpochNumber,
	) -> Result<McSlotNumber, EpochDerivationError> {
		let epochs_since_first_epoch = epoch
			.0
			.checked_sub(self.first_epoch_number)
			.ok_or(EpochDerivationError::EpochTooSmall)?;
		let slots_since_first_epoch = u64::from(epochs_since_first_epoch) * self.slots_per_epoch();
		Ok(McSlotNumber(slots_since_first_epoch + self.first_slot_number))
	}

	fn epoch_for_slot(&self, slot: McSlotNumber) -> Result<McEpochNumber, EpochDerivationError> {
		let slots_since_first_slot = slot
			.0
			.checked_sub(self.first_slot_number)
			.ok_or(EpochDerivationError::SlotTooSmall)?;
		let epochs_since_first_epoch =
			u32::try_from(slots_since_first_slot / self.slots_per_epoch())
				.map_err(|_| EpochDerivationError::EpochTooBig)?;
		Ok(McEpochNumber(self.first_epoch_number + epochs_since_first_epoch))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn read_epoch_config_from_env() {
		figment::Jail::expect_with(|jail| {
			set_mainchain_env(jail);
			assert_eq!(
				MainchainEpochConfig::read_from_env().expect("Should succeed"),
				MainchainEpochConfig {
					first_epoch_timestamp_millis: Timestamp::from_unix_millis(10),
					first_epoch_number: 100,
					epoch_duration_millis: Duration::from_millis(1000),
					first_slot_number: 42,
					slot_duration_millis: Duration::from_millis(1000)
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

	fn test_mc_epoch_config() -> MainchainEpochConfig {
		MainchainEpochConfig {
			first_epoch_timestamp_millis: Timestamp::from_unix_millis(1000),
			epoch_duration_millis: Duration::from_millis(1),
			first_epoch_number: 0,
			first_slot_number: 0,
			slot_duration_millis: Duration::from_millis(1000),
		}
	}

	fn testnet_mc_epoch_config() -> MainchainEpochConfig {
		MainchainEpochConfig {
			first_epoch_timestamp_millis: Timestamp::from_unix_millis(1596399616000),
			epoch_duration_millis: Duration::from_millis(5 * 24 * 60 * 60 * 1000),
			first_epoch_number: 75,
			first_slot_number: 0,
			slot_duration_millis: Duration::from_millis(1000),
		}
	}

	fn preprod_mc_epoch_config() -> MainchainEpochConfig {
		MainchainEpochConfig {
			first_epoch_timestamp_millis: Timestamp::from_unix_millis(1655798400000),
			epoch_duration_millis: Duration::from_millis(5 * 24 * 60 * 60 * 1000),
			first_epoch_number: 4,
			first_slot_number: 86400,
			slot_duration_millis: Duration::from_millis(1000),
		}
	}

	#[test]
	fn return_no_mainchain_epoch_on_timestamp_before_first_epoch() {
		assert_eq!(
			test_mc_epoch_config().timestamp_to_mainchain_epoch(Timestamp::from_unix_millis(100)),
			Err(EpochDerivationError::TimestampTooSmall)
		);
	}

	#[test]
	fn return_no_mainchain_slot_on_timestamp_before_first_epoch() {
		assert_eq!(
			test_mc_epoch_config()
				.timestamp_to_mainchain_slot_number(Timestamp::from_unix_millis(100)),
			Err(EpochDerivationError::TimestampTooSmall)
		);
	}

	#[test]
	fn return_right_mainchain_epoch_with_real_cardano_testnet_data() {
		assert_eq!(
			testnet_mc_epoch_config()
				.timestamp_to_mainchain_epoch(Timestamp::from_unix_millis(1637612455000))
				.expect("Should succeed"),
			McEpochNumber(170)
		);
	}

	#[test]
	fn return_right_mainchain_slot_with_real_cardano_testnet_data() {
		assert_eq!(
			testnet_mc_epoch_config()
				.timestamp_to_mainchain_slot_number(Timestamp::from_unix_millis(1637612455000))
				.expect("Should succeed"),
			41212839
		);
	}

	#[test]
	fn return_right_mainchain_slot_on_preprod() {
		assert_eq!(
			preprod_mc_epoch_config()
				.timestamp_to_mainchain_slot_number(Timestamp::from_unix_millis(1705091294000))
				.expect("Should succeed"),
			49379294
		);
	}

	#[test]
	fn first_slot_of_epoch_on_preprod() {
		assert_eq!(
			preprod_mc_epoch_config()
				.first_slot_of_epoch(McEpochNumber(117))
				.expect("Should succeed"),
			McSlotNumber(48902400)
		)
	}

	#[test]
	fn first_slot_of_epoch_on_preprod_epoch_too_small() {
		let config = preprod_mc_epoch_config();
		assert_eq!(
			config.first_slot_of_epoch(McEpochNumber(config.first_epoch_number - 1)),
			Err(EpochDerivationError::EpochTooSmall)
		)
	}

	#[test]
	fn epoch_for_slot_on_preprod() {
		let config = preprod_mc_epoch_config();
		assert_eq!(
			config.epoch_for_slot(McSlotNumber(48902399)).expect("Should succeed"),
			McEpochNumber(116)
		);
		assert_eq!(
			config.epoch_for_slot(McSlotNumber(48902400)).expect("Should succeed"),
			McEpochNumber(117)
		);
		assert_eq!(
			config.epoch_for_slot(McSlotNumber(48912400)).expect("Should succeed"),
			McEpochNumber(117)
		);
	}

	#[test]
	fn epoch_for_slot_on_preprod_slot_too_small() {
		let config = preprod_mc_epoch_config();
		assert_eq!(
			config.epoch_for_slot(McSlotNumber(config.first_slot_number - 1)),
			Err(EpochDerivationError::SlotTooSmall)
		);
		assert_eq!(config.epoch_for_slot(McSlotNumber(0)), Err(EpochDerivationError::SlotTooSmall))
	}
}
