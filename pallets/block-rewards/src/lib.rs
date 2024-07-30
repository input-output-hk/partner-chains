#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
pub mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use core::ops::Add;
	use frame_support::pallet_prelude::InherentIdentifier;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_block_rewards::*;
	use sp_std::vec::Vec;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type BeneficiaryId: Member + Parameter + MaxEncodedLen;

		/// Type of accumulated "reward" value
		/// It doesn't need to be a currency type
		type BlockRewardPoints: Member
			+ Parameter
			+ MaxEncodedLen
			+ Add<Self::BlockRewardPoints, Output = Self::BlockRewardPoints>;

		type GetBlockRewardPoints: GetBlockRewardPoints<Self::BlockRewardPoints>;
	}

	/// Beneficiary of the current block
	#[pallet::storage]
	pub type CurrentBlockBeneficiary<T: Config> = StorageValue<_, T::BeneficiaryId, OptionQuery>;

	/// Accumulated rewards of all beneficiaries since last payout
	#[pallet::storage]
	pub type PendingRewards<T: Config> =
		StorageMap<_, Twox64Concat, T::BeneficiaryId, T::BlockRewardPoints, OptionQuery>;

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;
		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let beneficiary = Self::get_beneficiary_from_inherent_data(data)
				.expect("Block beneficiary is not set");
			Some(Call::set_current_block_beneficiary { beneficiary })
		}

		fn check_inherent(_call: &Self::Call, _data: &InherentData) -> Result<(), Self::Error> {
			// The beneficiary provided by the block producer is always trusted
			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::set_current_block_beneficiary { .. })
		}

		fn is_inherent_required(_: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			Ok(Some(Self::Error::InherentRequired))
		}
	}

	impl<T: Config> Pallet<T> {
		fn get_beneficiary_from_inherent_data(
			data: &InherentData,
		) -> Result<T::BeneficiaryId, InherentError> {
			let beneficiary = data
				.get_data::<T::BeneficiaryId>(&Self::INHERENT_IDENTIFIER)
				.expect("❌ Block beneficiary inherent data is missing")
				.expect("❌ Block beneficiary inherent data is missing or invalid: {err:?}");
			Ok(beneficiary)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn set_current_block_beneficiary(
			origin: OriginFor<T>,
			beneficiary: T::BeneficiaryId,
		) -> DispatchResult {
			ensure_none(origin)?;
			CurrentBlockBeneficiary::<T>::put(beneficiary);
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(block: BlockNumberFor<T>) {
			let beneficiary = CurrentBlockBeneficiary::<T>::take()
				.expect("Block beneficiary is set before on_finalize; qed");
			let reward = T::GetBlockRewardPoints::get_block_reward();

			PendingRewards::<T>::mutate(&beneficiary, |old_reward| {
				*old_reward = match old_reward {
					None => Some(reward),
					Some(old_reward) => Some(old_reward.clone() + reward),
				}
			});

			log::info!("💵 Block {block:?} beneficiary is {beneficiary:?}")
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_rewards_and_clear() -> Vec<(T::BeneficiaryId, T::BlockRewardPoints)> {
			PendingRewards::<T>::drain().collect()
		}
	}
}
