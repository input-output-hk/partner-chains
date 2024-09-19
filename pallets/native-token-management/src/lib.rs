//! Pallet allowing Partner Chains to support movement of their native token from Cardano.
//!
//! Partner Chains Smart Contracts establish a notion of liquid and illiquid supply of the native
//! token on Cardano, represented as native tokens being either freely available in user accounts
//! or locked under a designated illiquid supply address. Movement of native tokens into the illiquid
//! supply on Cardano signals that an equivalent amount of tokens should be made available on the
//! Partner Chain.
//!
//! This pallet consumes inherent data containing information on the amount of native tokens newly
//! locked on Cardano and produces an inherent extrinsic to handle their movement. The specific
//! logic releasing the tokens is left for the Partner Chain developer to implement and is configured
//! in the pallet via the `TokenTransferHandler`.
//!
//! IMPORTANT: The mechanism implemented via the pallet in this crate is only concerned with the
//!            amount of tokens moved and does not attach any metadata to the transfers. In particular
//!            it is not a fully-featured token bridge and needs to be combined with a separate
//!            sender-receiver metadata channel to implement one.
//!
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sidechain_domain::*;
use sp_native_token_management::*;

#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "mock"))]
mod mock;

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
	pub type MainChainScriptsConfiguration<T: Config> =
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
			MainChainScriptsConfiguration::<T>::put(self.main_chain_scripts.clone());
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			Self::get_transfered_tokens_from_inherent_data(data)
				.filter(|data| data.token_amount.0 > 0)
				.map(|data| Call::transfer_tokens { token_amount: data.token_amount })
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let actual_transfer = match call {
				Call::transfer_tokens { token_amount } => token_amount.clone(),
				_ => return Ok(()),
			};

			let expected_transfer = match Self::get_transfered_tokens_from_inherent_data(data) {
				Some(data) => data.token_amount.clone(),
				None => {
					return Err(InherentError::UnexpectedTokenTransferInherent(actual_transfer))
				},
			};

			if expected_transfer != actual_transfer {
				return Err(InherentError::IncorrectTokenNumberTransfered(
					expected_transfer,
					actual_transfer,
				));
			}

			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::transfer_tokens { .. })
		}

		fn is_inherent_required(data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			Ok(Self::get_transfered_tokens_from_inherent_data(data)
				.filter(|data| data.token_amount.0 > 0)
				.map(|data| InherentError::TokenTransferNotHandled(data.token_amount.clone())))
		}
	}

	impl<T: Config> Pallet<T> {
		fn get_transfered_tokens_from_inherent_data(
			data: &InherentData,
		) -> Option<TokenTransferData> {
			data.get_data::<TokenTransferData>(&INHERENT_IDENTIFIER)
				.expect("Token transfer data is not encoded correctly")
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

		/// Changes the main chain scripts used for observing native token transfers.
		///
		/// This extrinsic must be run either using `sudo` or some other chain governance mechanism.
		#[pallet::call_index(1)]
		#[pallet::weight((1, DispatchClass::Normal))]
		pub fn set_main_chain_scripts(
			origin: OriginFor<T>,
			native_token_policy_id: PolicyId,
			native_token_asset_name: AssetName,
			illiquid_supply_validator_address: MainchainAddress,
		) -> DispatchResult {
			ensure_root(origin)?;
			let new_scripts = sp_native_token_management::MainChainScripts {
				native_token_policy_id,
				native_token_asset_name,
				illiquid_supply_validator_address,
			};
			MainChainScriptsConfiguration::<T>::put(new_scripts);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_main_chain_scripts() -> sp_native_token_management::MainChainScripts {
			MainChainScriptsConfiguration::<T>::get()
		}
	}
}
