//! Pallet that tracks information about incoming token bridge transfers observed on Cardano.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

/// Pallet benchmarking code
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

/// Weight types and default weight values
pub mod weights;

pub use pallet::*;
use sp_partner_chains_bridge::BridgeTransferV1;

/// Runtime logic for handling incoming token bridge transfers from Cardano
///
/// The chain builder should implement in accordance with their particular business rules and
/// ledger structure. Calls to all functions defined by this trait should not return any errors
/// as this would fail the block creation. Instead, any validation and business logic errors
/// should be handled gracefully inside the handler code.
pub trait TransferHandler<Recipient> {
	/// Should handle an incoming token transfer of `token_mount` tokens to `recipient`
	fn handle_incoming_transfer(_transfer: BridgeTransferV1<Recipient>);
}

/// No-op implementation of `TransferHandler` for unit type.
impl<Recipient> TransferHandler<Recipient> for () {
	fn handle_incoming_transfer(_transfer: BridgeTransferV1<Recipient>) {}
}

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::{ensure_none, pallet_prelude::OriginFor};
	use parity_scale_codec::MaxEncodedLen;
	use sp_partner_chains_bridge::{BridgeDataCheckpoint, TokenBridgeTransfersV1};
	use sp_partner_chains_bridge::{INHERENT_IDENTIFIER, InherentError, MainChainScripts};

	/// Current version of the pallet
	pub const PALLET_VERSION: u32 = 1;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Origin for governance extrinsic calls.
		///
		/// Typically the `EnsureRoot` type can be used unless a non-standard on-chain governance is used.
		type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Transfer recipient
		type Recipient: Member + Parameter + MaxEncodedLen;

		/// Handler for incoming token transfers
		type TransferHandler: TransferHandler<Self::Recipient>;

		/// Maximum number of transfers that can be handled in one block for each transfer type
		type MaxTransfersPerBlock: Get<u32>;

		/// Extrinsic weight information
		type WeightInfo: crate::weights::WeightInfo;

		/// Benchmark helper type used for running benchmarks
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: benchmarking::BenchmarkHelper<Self>;
	}

	/// Error type used by the pallet's extrinsics
	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::storage]
	pub type MainChainScriptsConfiguration<T: Config> =
		StorageValue<_, MainChainScripts, OptionQuery>;

	#[pallet::storage]
	pub type DataCheckpoint<T: Config> = StorageValue<_, BridgeDataCheckpoint, OptionQuery>;

	/// Genesis configuration of the pallet
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		/// Initial main chain scripts
		pub main_chain_scripts: Option<MainChainScripts>,
		#[allow(missing_docs)]
		pub _marker: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { main_chain_scripts: None, _marker: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			MainChainScriptsConfiguration::<T>::set(self.main_chain_scripts.clone());
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Inherent extrinsic that handles all incoming transfers in the current block
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn handle_transfers(
			origin: OriginFor<T>,
			transfers: BoundedVec<BridgeTransferV1<T::Recipient>, T::MaxTransfersPerBlock>,
			data_checkpoint: BridgeDataCheckpoint,
		) -> DispatchResult {
			ensure_none(origin)?;
			for transfer in transfers {
				T::TransferHandler::handle_incoming_transfer(transfer);
			}
			DataCheckpoint::<T>::put(data_checkpoint);
			Ok(())
		}

		/// Changes the main chain scripts used for observing native token transfers.
		///
		/// This extrinsic must be run either using `sudo` or some other chain governance mechanism.
		///
		/// *IMPORTANT*: Changing the main chain scripts will reset the data checkpoint stored in the
		/// pallet. This is to support a use case where a running Partner Chain needs to migrate to a
		/// new version of smart contracts which requires changing the observed addresses. It is assumed
		/// that a chain will always switch to a new set of addresses. Switching to a previously
		/// observed address __will cause previously handled transactions to be observed again__.
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn set_main_chain_scripts(
			origin: OriginFor<T>,
			new_scripts: MainChainScripts,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			if MainChainScriptsConfiguration::<T>::get() != Some(new_scripts.clone()) {
				MainChainScriptsConfiguration::<T>::put(new_scripts);
				DataCheckpoint::<T>::set(None);
			}
			Ok(())
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			Self::create_inherent_or_err(data).expect("Creating token bridge inherent failed")
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let Some(expected_call) = Self::create_inherent_or_err(data)? else {
				return Err(Self::Error::InherentNotExpected);
			};

			if *call != expected_call {
				return Err(Self::Error::IncorrectInherent);
			}

			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::handle_transfers { .. })
		}

		fn is_inherent_required(data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			match Self::decode_inherent_data(data) {
				None => Ok(None),
				Some(_) => Ok(Some(Self::Error::InherentRequired)),
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn decode_inherent_data(
			data: &InherentData,
		) -> Option<TokenBridgeTransfersV1<T::Recipient>> {
			data.get_data(&INHERENT_IDENTIFIER)
				.expect("Bridge inherent data is not encoded correctly")
		}

		fn create_inherent_or_err(data: &InherentData) -> Result<Option<Call<T>>, InherentError> {
			match Self::decode_inherent_data(data) {
				None => Ok(None),
				Some(data) => Ok(Some(Call::handle_transfers {
					transfers: data.transfers.try_into().map_err(|_| {
						InherentError::TooManyTransfers(T::MaxTransfersPerBlock::get())
					})?,
					data_checkpoint: data.data_checkpoint,
				})),
			}
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns current pallet version
		pub fn get_pallet_version() -> u32 {
			PALLET_VERSION
		}

		/// Returns the currently configured main chain scripts
		pub fn get_main_chain_scripts() -> Option<MainChainScripts> {
			MainChainScriptsConfiguration::<T>::get()
		}

		/// Returns the currently configured transfers per block limit
		pub fn get_max_transfers_per_block() -> u32 {
			T::MaxTransfersPerBlock::get()
		}

		/// Returns the current data checkpoint
		pub fn get_data_checkpoint() -> Option<BridgeDataCheckpoint> {
			DataCheckpoint::<T>::get()
		}
	}
}
