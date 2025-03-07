pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::BlockAuthor;
	use frame_support::pallet_prelude::*;
	use frame_system::ensure_none;
	use frame_system::pallet_prelude::OriginFor;
	use sidechain_domain::*;
	use sp_block_participation::BlockProductionData;
	use sp_inherents::IsFatalError;

	type ParticipationData = BlockProductionData<BlockAuthor, DelegatorKey>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_block_rewards::Config {}

	#[pallet::storage]
	#[pallet::unbounded]
	pub type LatestParticipationData<T: Config> = StorageValue<_, ParticipationData, OptionQuery>;

	impl<T: Config> sp_sidechain::OnNewEpoch for Pallet<T> {
		fn on_new_epoch(old_epoch: ScEpochNumber, _new_epoch: ScEpochNumber) -> sp_weights::Weight {
			let rewards = pallet_block_rewards::Pallet::<T>::get_rewards_and_clear();
			log::info!("Rewards accrued in epoch {old_epoch}: {rewards:?}");

			crate::RuntimeDbWeight::get().reads_writes(1, 1)
		}
	}

	impl<T: Config> pallet_native_token_management::TokenTransferHandler for Pallet<T> {
		fn handle_token_transfer(token_amount: NativeTokenAmount) -> DispatchResult {
			log::info!("💸 Registered transfer of {} native tokens", token_amount.0);
			Ok(())
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn handle_participation_data(
			origin: OriginFor<T>,
			data: ParticipationData,
		) -> DispatchResult {
			ensure_none(origin)?;
			log::info!("📊 Block participation inherent data released");
			LatestParticipationData::<T>::put(data);
			Ok(())
		}
	}

	#[derive(Clone, Debug, Encode, Decode)]
	pub enum InherentError {}
	impl IsFatalError for InherentError {
		fn is_fatal_error(&self) -> bool {
			true
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = *b"testhelp";

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let inherent = data
				.get_data::<ParticipationData>(&Self::INHERENT_IDENTIFIER)
				.expect("Block participation inherent data invalid")
				.map(|data| Self::Call::handle_participation_data { data });

			inherent
		}

		fn is_inherent(_call: &Self::Call) -> bool {
			true
		}

		fn is_inherent_required(_data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			Ok(None)
		}
	}
}
