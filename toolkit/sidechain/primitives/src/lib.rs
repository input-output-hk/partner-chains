//! Primitives for the Sidechain pallet
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

use frame_support::pallet_prelude::Weight;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sidechain_domain::{ScEpochNumber, ScSlotNumber, UtxoId};

#[cfg(test)]
mod tests;

/// Information about current Partner Chain slot and epoch.
#[deprecated(since = "1.8.0", note = "See deprecation notes for [GetSidechainStatus]")]
#[derive(TypeInfo, Clone, Encode, Decode)]
pub struct SidechainStatus {
	/// current Partner Chain epoch
	pub epoch: ScEpochNumber,
	/// current Partner Chain slot
	pub slot: ScSlotNumber,
	/// Number of slots per Partner Chain epoch
	pub slots_per_epoch: u32,
}

/// Handler to be called when new epoch starts
///
/// Instances of [OnNewEpoch] can be added to the Sidechain pallet to be called on new epoch.
pub trait OnNewEpoch {
	/// New epoch handler
	fn on_new_epoch(old_epoch: ScEpochNumber, new_epoch: ScEpochNumber) -> Weight;
}

impl OnNewEpoch for () {
	fn on_new_epoch(_old_epoch: ScEpochNumber, _new_epoch: ScEpochNumber) -> Weight {
		Weight::zero()
	}
}

macro_rules! on_new_epoch_tuple_impl {
	($first:ident, $($rest:ident),+) => {
		impl<$first, $($rest),+> OnNewEpoch for ($first, $($rest),+)
			where
				$first: OnNewEpoch,
				$($rest: OnNewEpoch),+
		{
			fn on_new_epoch(old_epoch: ScEpochNumber, new_epoch: ScEpochNumber) -> Weight {
				<$first as OnNewEpoch>::on_new_epoch(old_epoch, new_epoch)
					$(.saturating_add(<$rest as OnNewEpoch>::on_new_epoch(old_epoch, new_epoch)))+
			}
		}
	};
}
on_new_epoch_tuple_impl!(A, B);
on_new_epoch_tuple_impl!(A, B, C);
on_new_epoch_tuple_impl!(A, B, C, D);

#[allow(deprecated)]
mod api_declarations {
	use super::*;
	sp_api::decl_runtime_apis! {
		/// Runtime API for retrieving the Partner Chain's genesis UTXO
		pub trait GetGenesisUtxo {
			/// Returns the Partner Chain's genesis UTXO
			fn genesis_utxo() -> UtxoId;
		}

		/// Runtime API for getting information about current Partner Chain slot and epoch
		#[deprecated(since = "1.8.0", note = "Code that needs this data should define its own runtime API instead.")]
		pub trait GetSidechainStatus {
			/// Returns current Partner Chain slot and epoch
			fn get_sidechain_status() -> SidechainStatus;
		}
	}
}
pub use api_declarations::*;

/// Reads the genesis UTXO from the environment variable `GENESIS_UTXO`
#[cfg(feature = "std")]
pub fn read_genesis_utxo_from_env_with_defaults() -> Result<UtxoId, envy::Error> {
	/// This structure is needed to read sidechain params from the environment variables because the main
	/// type uses `rename_all = "camelCase"` serde option
	#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
	struct GenesisUtxoEnvConfiguration {
		pub genesis_utxo: UtxoId,
	}
	let raw = envy::from_env::<GenesisUtxoEnvConfiguration>()?;
	Ok(raw.genesis_utxo)
}
