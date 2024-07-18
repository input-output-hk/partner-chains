#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::inherent::IsFatalError;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sidechain_domain::NativeTokenAmount;
use sp_native_token_management::*;

#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "mock"))]
mod mock;

#[derive(Encode, Debug, PartialEq)]
pub enum InherentError {
	TokenTransferNotHandled,
	IncorrectTokenNumberTransfered,
	UnexpectedTokenTransferInherent,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

/// Interface for user-provided logic to handle native token transfers into the illiquid supply on the main chain.
///
/// The handler will be called with **the total sum** of transfers since the previous partner chain block.
pub trait TokenTransferHandler {
	fn handle_token_transfer(token_amount: NativeTokenAmount) -> DispatchResult;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type TokenTransferHandler: TokenTransferHandler;
	}

	#[pallet::event]
	pub enum Event<T: Config> {
		TokensTransfered(NativeTokenAmount),
	}

	#[pallet::storage]
	pub type MainChainScripts<T: Config> =
		StorageValue<_, sp_native_token_management::MainChainScripts, ValueQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub main_chain_scripts: sp_native_token_management::MainChainScripts,
		pub _marker: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			MainChainScripts::<T>::put(self.main_chain_scripts.clone());
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			Self::get_transfered_tokens_from_inherent_data(data)
				.map(|data| Call::transfer_tokens { token_amount: data.token_amount })
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let Some(token_transfer) = Self::get_transfered_tokens_from_inherent_data(data) else {
				return Err(InherentError::UnexpectedTokenTransferInherent);
			};
			let Call::transfer_tokens { token_amount } = call else {
				unreachable!("There is no other extrinsic in this pallet");
			};
			if token_transfer.token_amount != *token_amount {
				return Err(InherentError::IncorrectTokenNumberTransfered);
			}

			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::transfer_tokens { .. })
		}

		fn is_inherent_required(data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			Ok(Self::get_transfered_tokens_from_inherent_data(data)
				.map(|_| InherentError::TokenTransferNotHandled))
		}
	}

	impl<T: Config> Pallet<T> {
		fn get_transfered_tokens_from_inherent_data(
			data: &InherentData,
		) -> Option<TokenTransferData> {
			data.get_data::<TokenTransferData>(&INHERENT_IDENTIFIER)
				.expect("Token transfer data not correctly encoded")
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn transfer_tokens(
			origin: OriginFor<T>,
			token_amount: NativeTokenAmount,
		) -> DispatchResult {
			ensure_none(origin)?;
			T::TokenTransferHandler::handle_token_transfer(token_amount)
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_main_chain_scripts() -> sp_native_token_management::MainChainScripts {
			MainChainScripts::<T>::get()
		}
	}
}
