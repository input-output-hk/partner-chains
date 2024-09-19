#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub mod runtime_api_client;

use core::ops::Rem;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sidechain_domain::{ScEpochNumber, ScSlotNumber};
pub use sp_consensus_slots::{Slot, SlotDuration};
use sp_core::offchain::Timestamp;

#[derive(Clone, Copy, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SlotsPerEpoch(pub u32);

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
	#[cfg(feature = "std")]
	pub fn read_from_env() -> Result<Self, envy::Error> {
		#[derive(Serialize, Deserialize)]
		struct SlotsPerEpochEnvConfig {
			#[serde(default = "default_slots_per_epoch")]
			slots_per_epoch: u32,
		}

		let raw = envy::from_env::<SlotsPerEpochEnvConfig>()?;
		Ok(Self(raw.slots_per_epoch))
	}

	pub fn epoch_number(&self, slot: Slot) -> ScEpochNumber {
		epoch_number(slot, self.0)
	}

	pub fn epoch_number_from_sc_slot(&self, slot: ScSlotNumber) -> ScEpochNumber {
		epoch_number(Slot::from(slot.0), self.0)
	}

	pub fn first_slot_number(&self, epoch: ScEpochNumber) -> Slot {
		first_slot_number(epoch, self.0)
	}
	pub fn slot_number_in_epoch(&self, slot: Slot) -> u32 {
		slot_number_in_epoch(slot, self.0)
	}
}

#[derive(Clone, Debug, Encode, Decode, TypeInfo)]
pub struct ScSlotConfig {
	pub slots_per_epoch: SlotsPerEpoch,
	pub slot_duration: SlotDuration,
}

impl ScSlotConfig {
	pub fn epoch_number(&self, slot: Slot) -> ScEpochNumber {
		self.slots_per_epoch.epoch_number(slot)
	}
	pub fn first_slot_number(&self, epoch: ScEpochNumber) -> Slot {
		self.slots_per_epoch.first_slot_number(epoch)
	}
	pub fn slot_from_timestamp(&self, timestamp: u64) -> Slot {
		Slot::from_timestamp(timestamp.into(), self.slot_duration)
	}
	pub fn epoch_start_time(&self, epoch: ScEpochNumber) -> Option<Timestamp> {
		let slot = self.first_slot_number(epoch);
		self.slot_duration
			.as_millis()
			.checked_mul(*slot)
			.map(Timestamp::from_unix_millis)
	}
}

pub fn epoch_number(slot: Slot, slots_per_epoch: u32) -> ScEpochNumber {
	ScEpochNumber(*slot / u64::from(slots_per_epoch))
}

/// Get the first slot number of the epoch `epoch`
pub fn first_slot_number(epoch: ScEpochNumber, slots_per_epoch: u32) -> Slot {
	Slot::from(epoch.0 * slots_per_epoch as u64)
}

pub fn slot_number_in_epoch(slot: Slot, slots_per_epoch: u32) -> u32 {
	u32::try_from(slot.rem(u64::from(slots_per_epoch)))
		.expect("slots_per_epoch is u32, thus any modulo reminder of it is also u32")
}

pub fn is_last_slot_of_an_epoch(slot: Slot, slots_per_epoch: u32) -> bool {
	slot_number_in_epoch(slot, slots_per_epoch) == slots_per_epoch - 1
}

pub enum Error {
	OverflowError,
}

sp_api::decl_runtime_apis! {
	pub trait SlotApi {
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
