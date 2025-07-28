//! Pallet allowing Partner Chains to support movement of their native token from Cardano.
//!
//! # Context and purpose of this pallet
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
//! in the pallet via the [TokenTransferHandler] trait.
//!
//! *IMPORTANT*: The mechanism implemented by this pallet is only concerned with the amount of tokens
//!              moved and does not attach any metadata to the transfers. In particular it is not a
//!              fully-featured token bridge and needs to be combined with a separate sender-receiver
//!              metadata channel to implement one.
//!
//! # Usage
//!
//! ## Defining a transfer handler
//!
//! The main purpose of the pallet is to trigger user-defined runtime logic whenever a new batch of
//! tokens is observed to have been locked on Cardano. To do that, the Partner Chain builder should
//! define a type implementing the [TokenTransferHandler] trait, eg.:
//!
//! ```rust
//! use sidechain_domain::NativeTokenAmount;
//! use frame_support::pallet_prelude::DispatchResult;
//!
//! pub struct TransferHandler;
//! impl pallet_native_token_management::TokenTransferHandler for TransferHandler {
//! 	fn handle_token_transfer(token_amount: NativeTokenAmount) -> DispatchResult {
//! 		log::info!("💸 Registered transfer of {} native tokens", token_amount.0);
//! 		Ok(())
//! 	}
//! }
//! ```
//!
//! ## Adding to the runtime
//!
//! Aside from the transfer handler, the pallet requires minimal runtime configuration: the runtime event
//! type, origin for governance calls implementing [EnsureOrigin] and weights:
//!
//! ```rust,ignore
//! impl pallet_native_token_management::Config for Runtime {
//! 	type RuntimeEvent = RuntimeEvent;
//! 	type TokenTransferHandler = TransferHandler;
//! 	type MainChainScriptsOrigin = frame_system::EnsureRoot<Self::AccountId>;
//! 	type WeightInfo = pallet_native_token_management::weights::SubstrateWeight<Runtime>;
//! }
//! ```
//!
//! Keep in mind that if the handler logic has to perform storage operations, the pallet's benchmarks
//! should be rerun. Otherwise default weights are provided in [crate::weights].
//!
//! ## Script configuration
//!
//! For token transfers to be observed, the pallet must be configured with correct Cardano addresses and
//! scripts used to idendify them in the ledger. These scripts can be set in two ways: through genesis
//! configuration if the pallet is present in the initial runtime of a chain; or via a governance action
//! using the [Pallet::set_main_chain_scripts] extrinsic.
//!
//! ### Genesis configuratin
//!
//! Initial main chain scripts can be set in the genesis configuration, like so:
//! ```json
//! {
//!   "nativeTokenManagement": {
//!     "mainChainScripts": {
//!       "illiquid_supply_validator_address": "0x616464725f74657374317772687674767833663067397776397278386b66716336306a7661336530376e71756a6b32637370656b76346d717339726a64767a",
//!       "native_token_asset_name": "0x5043546f6b656e44656d6f",
//!       "native_token_policy_id": "0xada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4"
//!     }
//!   }
//! }
//! ```
//!
//! Note that the `mainChainScripts` field is optional. If it is left empty, the pallet will stay inactive
//! until configuration is set later.
//!
//! ### Main chain scripts extrinsic
//!
//! Once the chain is already started, to set initial main chain scripts to a newly added pallet, or to
//! change the existing ones, the [Pallet::set_main_chain_scripts] extrinsic must be submitted through on-chain
//! governance mechanism like `sudo` or `pallet_democracy`. Who exactly can submit this extrinsic is
//! controlled by the [Config::MainChainScriptsOrigin] field of the pallet's configuration, but for security
//! it must be a trusted entity.
//!
//! #### Initialization state of the pallet
//!
//! The pallet tracks its own initialization state through the [Initialized] storage flag. This information
//! is necessary for it to correctly observe historical data and the state is reset every time main chain
//! scripts are changed in the pallet. This allows the Partner Chain governance to switch to new versions
//! of the smart contracts. However, some consideration must be taken while changing the scripts:
//! 1. This mechanism can not handle changing the main chain scripts to values that were used before.
//!    Doing so will cause some transfers to be registered again, resulting in potential double-spend.
//!    This means that a script version roll-back is not possible.
//! 2. Moving funds from an old illiquid supply address to a new one requires unlocking them and re-locking
//!    at the new address, resulting in a new transfer being observed. The logic handling the token movement,
//!    including the transfer handler, must be able to handle this unlock-relock behaviour if a Partner Chain
//!    governance wishes to migrate tokens to the new address.
//!
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sidechain_domain::*;
use sp_native_token_management::*;

mod benchmarking;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

pub mod weights;
pub use weights::WeightInfo;

/// Interface for user-provided logic to handle native token transfers into the illiquid supply on the main chain.
///
/// The handler will be called with **the total sum** of transfers since the previous partner chain block.
pub trait TokenTransferHandler {
	/// New transfer even handler
	fn handle_token_transfer(token_amount: NativeTokenAmount) -> DispatchResult;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Origin for governance calls
		type MainChainScriptsOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Event handler for incoming native token transfers
		type TokenTransferHandler: TokenTransferHandler;

		/// Weight information for this pallet's extrinsics
		type WeightInfo: WeightInfo;
	}

	/// Events emitted by this pallet
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Signals that a new native token transfer has been processed by the pallet
		TokensTransfered(NativeTokenAmount),
	}

	/// Error type used by the pallet's extrinsics
	#[pallet::error]
	pub enum Error<T> {
		/// Indicates that the inherent was called while there was no main chain scripts set in the
		/// pallet's storage. This is indicative of a programming bug.
		CalledWithoutConfiguration,
		/// Indicates that the inherent was called a second time in the same block
		TransferAlreadyHandled,
	}

	#[pallet::storage]
	pub type MainChainScriptsConfiguration<T: Config> =
		StorageValue<_, sp_native_token_management::MainChainScripts, OptionQuery>;

	/// Stores the pallet's initialization state.
	///
	/// The pallet is considered initialized if its inherent has been successfuly called at least once since
	/// genesis or the last invocation of [Pallet::set_main_chain_scripts].
	#[pallet::storage]
	pub type Initialized<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Transient storage containing the amount of native token transfer registered in the current block.
	///
	/// Any value in this storage is only present during execution of a block and is emptied on block finalization.
	#[pallet::storage]
	pub type TransferedThisBlock<T: Config> = StorageValue<_, NativeTokenAmount, OptionQuery>;

	/// Genesis configuration of the pallet
	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// Initial main chain scripts
		pub main_chain_scripts: Option<sp_native_token_management::MainChainScripts>,
		#[allow(missing_docs)]
		pub _marker: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			MainChainScriptsConfiguration::<T>::set(self.main_chain_scripts.clone());
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
				Call::transfer_tokens { token_amount } => *token_amount,
				_ => return Ok(()),
			};

			let expected_transfer = match Self::get_transfered_tokens_from_inherent_data(data) {
				Some(data) => data.token_amount,
				None => {
					return Err(InherentError::UnexpectedTokenTransferInherent(actual_transfer));
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
				.map(|data| InherentError::TokenTransferNotHandled(data.token_amount)))
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
		/// Inherent that registers new native token transfer from the Cardano main chain and triggers
		/// the handler configured in [Config::TokenTransferHandler].
		///
		/// Arguments:
		/// - `token_amount`: the total amount of tokens transferred since the last invocation of the inherent
		#[pallet::call_index(0)]
		#[pallet::weight((T::WeightInfo::transfer_tokens(), DispatchClass::Mandatory))]
		pub fn transfer_tokens(
			origin: OriginFor<T>,
			token_amount: NativeTokenAmount,
		) -> DispatchResult {
			ensure_none(origin)?;
			ensure!(
				MainChainScriptsConfiguration::<T>::exists(),
				Error::<T>::CalledWithoutConfiguration
			);
			ensure!(!TransferedThisBlock::<T>::exists(), Error::<T>::TransferAlreadyHandled);
			Initialized::<T>::mutate(|initialized| {
				if !*initialized {
					*initialized = true
				}
				true
			});
			TransferedThisBlock::<T>::put(token_amount);
			Self::deposit_event(Event::TokensTransfered(token_amount));
			T::TokenTransferHandler::handle_token_transfer(token_amount)
		}

		/// Changes the main chain scripts used for observing native token transfers.
		///
		/// This extrinsic must be run either using `sudo` or some other chain governance mechanism.
		#[pallet::call_index(1)]
		#[pallet::weight((T::WeightInfo::set_main_chain_scripts(), DispatchClass::Normal))]
		pub fn set_main_chain_scripts(
			origin: OriginFor<T>,
			native_token_policy_id: PolicyId,
			native_token_asset_name: AssetName,
			illiquid_supply_validator_address: MainchainAddress,
		) -> DispatchResult {
			T::MainChainScriptsOrigin::ensure_origin(origin)?;
			let new_scripts = sp_native_token_management::MainChainScripts {
				native_token_policy_id,
				native_token_asset_name,
				illiquid_supply_validator_address,
			};
			MainChainScriptsConfiguration::<T>::put(new_scripts);
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// A dummy `on_initialize` to return the amount of weight that `on_finalize` requires to
		/// execute.
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			T::WeightInfo::on_finalize()
		}

		fn on_finalize(_block: BlockNumberFor<T>) {
			TransferedThisBlock::<T>::kill();
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns the main chain scripts currently configured in the pallet
		pub fn get_main_chain_scripts() -> Option<sp_native_token_management::MainChainScripts> {
			MainChainScriptsConfiguration::<T>::get()
		}
		/// Returns the current initialization status of the pallet
		pub fn initialized() -> bool {
			Initialized::<T>::get()
		}
	}
}
