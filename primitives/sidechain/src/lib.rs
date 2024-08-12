#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::Weight;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sidechain_domain::{ScEpochNumber, ScSlotNumber};
#[cfg(feature = "std")]
use sp_runtime::traits::Block as BlockT;

#[cfg(feature = "std")]
pub mod query;

#[cfg(test)]
mod tests;

#[derive(TypeInfo, Clone, Encode, Decode)]
pub struct SidechainStatus {
	pub epoch: ScEpochNumber,
	pub slot: ScSlotNumber,
	pub slots_per_epoch: u32,
}

pub trait OnNewEpoch {
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

sp_api::decl_runtime_apis! {
	pub trait GetSidechainParams<P: parity_scale_codec::Decode> {
		fn sidechain_params() -> P;
	}
	pub trait GetSidechainStatus {
		fn get_sidechain_status() -> SidechainStatus;
	}
}

#[cfg(feature = "std")]
pub trait SidechainApi<Block: BlockT, P: parity_scale_codec::Decode>:
	GetSidechainStatus<Block> + GetSidechainParams<Block, P>
{
}

#[cfg(feature = "std")]
impl<
		Block: BlockT,
		P: parity_scale_codec::Decode,
		T: GetSidechainParams<Block, P> + GetSidechainStatus<Block>,
	> SidechainApi<Block, P> for T
{
}
