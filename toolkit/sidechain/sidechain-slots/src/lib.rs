//! This crate provides types and logic for handling Partner Chain slots.
//!
//! Partner Chain slots are grouped into epochs of equal length.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

#[cfg(feature = "std")]
pub mod runtime_api_client;

use core::ops::Rem;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use sidechain_domain::{ScEpochNumber, ScSlotNumber};
pub use sp_consensus_slots::{Slot, SlotDuration};
use sp_core::offchain::Timestamp;

/// Number of slots in each Partner Chain epoch
///
/// This value should satisfy the following property:
/// > `main_chain_epoch_duration` % (`slots_per_epoch` * `slot_duration`)  == 0
/// that is, Cardano main chain's epoch boundaries should always coincide with
/// a Partner Chain epoch boundary, or in other words PC epochs should perfectly
/// cover a Cardano epoch.
#[derive(Clone, Copy, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SlotsPerEpoch(pub u32);

/// Default number of slots per epoch.
///
/// This value is used by [SlotsPerEpoch::read_from_env] when no other value is set in the environment.
pub fn default_slots_per_epoch() -> u32 {
	60
}

impl Default for SlotsPerEpoch {
	/// Set to 60 to maintain backwards compatibility with existing chains.
	fn default() -> Self {
		SlotsPerEpoch(60)
	}
}

impl SlotsPerEpoch {
	/// Reads [SlotsPerEpoch] from the environment variable `SLOTS_PER_EPOCH` with default.
	#[cfg(all(feature = "std", feature = "serde"))]
	pub fn read_from_env() -> Result<Self, envy::Error> {
		#[derive(Serialize, Deserialize)]
		struct SlotsPerEpochEnvConfig {
			#[serde(default = "default_slots_per_epoch")]
			slots_per_epoch: u32,
		}

		let raw = envy::from_env::<SlotsPerEpochEnvConfig>()?;
		Ok(Self(raw.slots_per_epoch))
	}

	/// Returns the epoch number of `slot`
	pub fn epoch_number(&self, slot: Slot) -> ScEpochNumber {
		epoch_number(slot, self.0)
	}

	/// Returns the epoch number of `slot`
	pub fn epoch_number_from_sc_slot(&self, slot: ScSlotNumber) -> ScEpochNumber {
		epoch_number(Slot::from(slot.0), self.0)
	}

	/// Returns the number of first slot in `epoch`
	pub fn first_slot_number(&self, epoch: ScEpochNumber) -> Slot {
		first_slot_number(epoch, self.0)
	}

	/// Returns the number of `slot` within its epoch
	pub fn slot_number_in_epoch(&self, slot: Slot) -> u32 {
		slot_number_in_epoch(slot, self.0)
	}
}

/// Slot and epoch configuration for a Partner Chain
#[derive(Clone, Debug, Encode, Decode, TypeInfo)]
pub struct ScSlotConfig {
	/// Number of slots per Partner Chain epoch
	pub slots_per_epoch: SlotsPerEpoch,
	/// Duration of a single Partner Chain slot
	pub slot_duration: SlotDuration,
}

impl ScSlotConfig {
	/// Returns the epoch number of `slot`
	pub fn epoch_number(&self, slot: Slot) -> ScEpochNumber {
		self.slots_per_epoch.epoch_number(slot)
	}

	/// Returns the number of first slot of `epoch`
	pub fn first_slot_number(&self, epoch: ScEpochNumber) -> Slot {
		self.slots_per_epoch.first_slot_number(epoch)
	}

	/// Returns the slot number that contains `timestamp`
	pub fn slot_from_timestamp(&self, timestamp: u64) -> Slot {
		Slot::from_timestamp(timestamp.into(), self.slot_duration)
	}

	/// Returns the start timestamp of `epoch`
	pub fn epoch_start_time(&self, epoch: ScEpochNumber) -> Option<Timestamp> {
		self.first_slot_number(epoch)
			.timestamp(self.slot_duration)
			.map(|s| Timestamp::from_unix_millis(s.as_millis()))
	}
}

/// Returns the epoch number for `slot` given `slots_per_epoch`
pub fn epoch_number(slot: Slot, slots_per_epoch: u32) -> ScEpochNumber {
	ScEpochNumber(*slot / u64::from(slots_per_epoch))
}

/// Get the first slot number of the epoch `epoch`
pub fn first_slot_number(epoch: ScEpochNumber, slots_per_epoch: u32) -> Slot {
	Slot::from(epoch.0 * slots_per_epoch as u64)
}

/// Returns the number of `slot` within its epoch given `slots_per_epoch`
pub fn slot_number_in_epoch(slot: Slot, slots_per_epoch: u32) -> u32 {
	u32::try_from(slot.rem(u64::from(slots_per_epoch)))
		.expect("slots_per_epoch is u32, thus any modulo reminder of it is also u32")
}

/// Checks whether `slot` is the last slot of its epoch given `slots_per_epoch`
pub fn is_last_slot_of_an_epoch(slot: Slot, slots_per_epoch: u32) -> bool {
	slot_number_in_epoch(slot, slots_per_epoch) == slots_per_epoch - 1
}

/// Error type returnes by epoch and slot handling functions in this crate
pub enum Error {
	/// Indicates that an integer overflow occured during epoch calculation
	OverflowError,
}

sp_api::decl_runtime_apis! {
	/// Runtime API serving slot configuration
	pub trait SlotApi {
		/// Returns the current slot configuration
		fn slot_config() -> ScSlotConfig;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;

	prop_compose! {
		fn arb_slot()(slot_number in 0..u64::MAX) -> Slot {
			Slot::from(slot_number)
		}
	}

	proptest! {
		#[test]
		fn slot_number_is_slot_modulo_slots_per_epoch(slot in arb_slot(), slots_per_epoch in 1..u32::MAX) {
			let expected =u32::try_from(*slot % u64::from(slots_per_epoch)).unwrap();
			assert_eq!(expected, slot_number_in_epoch(slot, slots_per_epoch))
		}
	}
}
