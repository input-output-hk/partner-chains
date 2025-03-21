use frame_support::pallet_prelude::{OptionQuery, ValueQuery, Zero};
use frame_support::{storage_alias, BoundedVec, CloneNoBound};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[derive(CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxValidators))]
pub struct LegacyCommitteeInfo<
	ScEpochNumber: Clone,
	AuthorityId: Clone,
	AuthorityKeys: Clone,
	MaxValidators,
> {
	pub epoch: ScEpochNumber,
	pub committee: BoundedVec<(AuthorityId, AuthorityKeys), MaxValidators>,
}

impl<ScEpochNumber, AuthorityId, AuthorityKeys, MaxValidators> Default
	for LegacyCommitteeInfo<ScEpochNumber, AuthorityId, AuthorityKeys, MaxValidators>
where
	AuthorityId: Clone,
	AuthorityKeys: Clone,
	ScEpochNumber: Clone + Zero,
{
	fn default() -> Self {
		Self { epoch: ScEpochNumber::zero(), committee: BoundedVec::new() }
	}
}

#[storage_alias]
pub type CurrentCommittee<T: crate::pallet::Config> = StorageValue<
	crate::Pallet<T>,
	LegacyCommitteeInfo<
		<T as crate::pallet::Config>::ScEpochNumber,
		<T as crate::pallet::Config>::AuthorityId,
		<T as crate::pallet::Config>::AuthorityKeys,
		<T as crate::pallet::Config>::MaxValidators,
	>,
	ValueQuery,
>;

#[storage_alias]
pub type NextCommittee<T: crate::pallet::Config> = StorageValue<
	crate::Pallet<T>,
	LegacyCommitteeInfo<
		<T as crate::pallet::Config>::ScEpochNumber,
		<T as crate::pallet::Config>::AuthorityId,
		<T as crate::pallet::Config>::AuthorityKeys,
		<T as crate::pallet::Config>::MaxValidators,
	>,
	OptionQuery,
>;
