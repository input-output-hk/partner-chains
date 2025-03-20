#![allow(deprecated)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::BlockAuthor;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::OriginFor;
	use frame_system::{ensure_none, ensure_root};
	use sidechain_domain::*;
	use sp_block_participation::BlockProductionData;
	use sp_inherents::IsFatalError;

	type ParticipationData = BlockProductionData<BlockAuthor, DelegatorKey>;

	pub const DEFAULT_PARTICIPATION_DATA_RELEASE_PERIOD: u64 = 30;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	#[pallet::unbounded]
	pub type LatestParticipationData<T: Config> = StorageValue<_, ParticipationData, OptionQuery>;

	#[pallet::type_value]
	pub fn DefaultParticipationDataReleasePeriod<T: Config>() -> u64 {
		DEFAULT_PARTICIPATION_DATA_RELEASE_PERIOD
	}

	#[pallet::storage]
	pub type ParticipationDataReleasePeriod<T: Config> =
		StorageValue<_, u64, ValueQuery, DefaultParticipationDataReleasePeriod<T>>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub participation_data_release_period: u64,
		pub _phantom: PhantomData<T>,
	}
	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			ParticipationDataReleasePeriod::<T>::put(self.participation_data_release_period);
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn should_release_participation_data(
			slot: sidechain_slots::Slot,
		) -> Option<sidechain_slots::Slot> {
			if *slot % ParticipationDataReleasePeriod::<T>::get() == 0 {
				Some(slot)
			} else {
				None
			}
		}
	}

	impl<T: Config> sp_sidechain::OnNewEpoch for Pallet<T> {
		fn on_new_epoch(
			_old_epoch: ScEpochNumber,
			_new_epoch: ScEpochNumber,
		) -> sp_weights::Weight {
			crate::RuntimeDbWeight::get().reads_writes(0, 0)
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

		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn set_block_participation_data_release_period(
			origin: OriginFor<T>,
			period: u64,
		) -> DispatchResult {
			ensure_root(origin)?;
			log::info!("📊 Block participation data release period changed to {period}");
			ParticipationDataReleasePeriod::<T>::put(period);
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
