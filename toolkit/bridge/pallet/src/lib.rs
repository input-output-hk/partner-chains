//! Pallet that tracks information about incoming token bridge transfers observed on Cardano.
//!
//! # Purpose of this pallet
//!
//! This pallet implements runtime logic necessary for Partner Chains to receive
//! token transfers from Cardano using the trustless token bridge. It exposes a
//! callback API for chain builders to hook their own transfer handling logic into
//! the pallet according to their business and ledger rules.
//!
//! # Working overview
//!
//! Bridge transfers are initiated by transactions on Cardano that create UTXOs
//! on the illiquid circulating supply (ICP) validator address, each containing
//! a datum which marks them as transfer UTXOs. The observability layer of a
//! Partner Chain node registers creation of these UTXOs and classifies them
//! either as *user transfers*, ie. transfers sent by normal chain users to a
//! Partner Chain address specified by the user; or special *reserve transfers*,
//! which are a mechanism for a Partner Chain to gradually move their token
//! reserve from Cardano to its own ledger.
//!
//! Newly observed and classified bridge transfers are provided to the runtime
//! as inherent data. Based on this data, the pallet creates an inherent
//! extrinsic to handle them in the runtime during block production. This
//! inherent does not process the transfers directly, and instead calls the
//! handler provided by the particular Partner Chain's builders. This allows
//! the pallet to not assume anything about the ledger structure and logic of
//! the Partner Chain.
//!
//! # Usage
//!
//! ## Define the recipient type
//!
//! All user transfers handler by the pallet are addressed to a recipient
//! specified in the datum of the transfer UTXO. This recipient can be any
//! type that can be encoded and decoded as a Plutus byte string. A natural
//! choice would be the account address used in the Partner Chain runtime,
//! but a different type can be chosen as needed.
//!
//! ## Implement the transfer handler
//!
//! Because the Substrate framework leaves the developers a lot of freedom in
//! structuring their ledger and defining runtime logic, the pallet does not
//! handle the transfers by itself. Instead, it must be configured with a
//! [TransferHandler] instance by the Partner Chain builder.
//!
//! This handler is expected to never fail and handle any errors internally,
//! unless there exists a case in which the chain should very deliberately
//! be unable to produce a block. In practice, this means that any invalid
//! transfers should be either discarded or saved for reprocessing later.
//!
//! A minimal example for a runtime that uses `pallet_balances` and `AccountId32`
//! as its recipient type could look like this:
//!
//! ```rust,ignore
//! pub struct BridgeTransferHelper;
//!
//! impl pallet_partner_chains_bridge::TransferHandler<AccountId32> for BridgeTransferHelper {
//! 	fn handle_incoming_transfer(transfer: BridgeTransferV1<AccountId32>) {
//! 		match transfer {
//! 			BridgeTransferV1::InvalidTransfer { token_amount, utxo_id } => {
//! 				log::warn!("⚠️ Discarded an invalid transfer of {token_amount} (utxo {utxo_id})");
//! 			},
//! 			BridgeTransferV1::UserTransfer { token_amount, recipient } => {
//! 				log::info!("💸 Registered a tranfer of {token_amount} to {recipient:?}");
//! 				let _ = Balances::deposit_creating(&recipient, token_amount.into());
//! 			},
//! 			BridgeTransferV1::ReserveTransfer { token_amount } => {
//! 				log::info!("🏦 Registered a reserve transfer of {token_amount}.");
//! 				let _ = Balances::deposit_creating(&T::ReserveAccount::get(), token_amount.into());
//! 			},
//! 		}
//! 	}
//! }
//! ```rust
//!
//! For runtimes that require more complex transfer handling logic, it is a good
//! practice to create a dedicated pallet in the runtime and have it implement
//! [TransferHandler], so that any relevant state and configuration can be stored
//! together.
//!
//! ## Adding the pallet to the runtime
//!
//! Add the pallet to your runtime's [construct_runtime] and configure it by supplying
//! all relevant types from your runtime:
//!
//! ```rust,ignore
//! parameter_types! {
//!     pub const MaxTransfersPerBlock: u32 = 256;
//! }
//!
//! impl pallet_partner_chains_bridge::Config for Runtime {
//! 	type GovernanceOrigin = EnsureRoot<Runtime>;
//! 	type Recipient = AccountId;
//! 	type TransferHandler = BridgeTransferHelper;
//! 	type MaxTransfersPerBlock = MaxTransfersPerBlock;
//! 	type WeightInfo = ();
//!
//! 	#[cfg(feature = "runtime-benchmarks")]
//! 	type BenchmarkHelper = ();
//! }
//! ```
//!
//! In particular, the pallet needs to be configured with the value  `MaxTransfersPerBlock`,
//! which determines the upper bound on the number of transactions that can be processed
//! per block. All outstanding transfers beyond that limit will be processed in subsequent
//! block. It is important to select a value high enough to guarantee that the chain will
//! be able to keep up with the volume of transfers coming in.
//!
//! The last thing to implement in the runtime is the runtime API used by the observability
//! layer to access the configuration stored in the pallet. This is straightforward and
//! involves only calling methods defined on the pallet:
//!
//! ```rust,ignore
//! impl sp_partner_chains_bridge::TokenBridgeIDPRuntimeApi<Block> for Runtime {
//! 	fn get_pallet_version() -> u32 {
//! 		Bridge::get_pallet_version()
//! 	}
//! 	fn get_main_chain_scripts() -> Option<BridgeMainChainScripts> {
//! 		Bridge::get_main_chain_scripts()
//! 	}
//! 	fn get_max_transfers_per_block() -> u32 {
//! 		Bridge::get_max_transfers_per_block()
//! 	}
//! 	fn get_last_data_checkpoint() -> Option<BridgeDataCheckpoint> {
//! 		Bridge::get_data_checkpoint()
//! 	}
//! }
//! ```
//!
//! ## Providing genesis configuration
//!
//! The pallet's genesis configuration only consists of optional values of
//! the main chain scripts, that can be set after chain start. These scripts
//! point the observability layer to the correct addresses and token asset
//! to observe on Cardano. If they are left empty, the pallet and the
//! observability components will be incactive until they are supplied in
//! the future.
//!
//! An example of a bridge pallet section in a genesis config JSON would
//! look like this:
//! ```json
//! {
//!     "bridge": {
//!       "mainChainScripts": {
//!         "illiquid_circulation_supply_validator_address": "addr_test1wzzyc3mcqh4phq0pa827dn756lfd045lzh3tgr9mt5p2ayqpxp55c",
//!         "token_asset_name": "0x5043546f6b656e44656d6f",
//!         "token_policy_id": "0xada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4"
//!       }
//! 	}
//! }
//! ```
//!
//! When programmatically assembling the genesis config, a utility function
//! is supplied for reading the main chain script values from environment:
//!
//! ```rust
//! # use pallet_partner_chains_bridge::{ Config, GenesisConfig };
//! # use sp_partner_chains_bridge::MainChainScripts;
//! # fn create_genesis_config<T: Config>() -> GenesisConfig<T> {
//! 	GenesisConfig {
//! 		main_chain_scripts: MainChainScripts::read_from_env().ok(),
//! 		..Default::default()
//! 	}
//! # }
//! ```
//!
//! See [sp_partner_chains_bridge::MainChainScripts::read_from_env] for details.
//!
//! ## Supplying observability data
//!
//! See documentation of [sp_partner_chains_bridge] for instructions on adding
//! the observability data source to your node and connecting it to the pallet.
//!
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

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::weights::WeightInfo;
	use frame_support::pallet_prelude::*;
	use frame_system::{ensure_none, pallet_prelude::OriginFor};
	use parity_scale_codec::MaxEncodedLen;
	use sidechain_domain::UtxoId;
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
		/// The initial data checkpoint. Chain Genesis UTXO is a good candidate for it.
		pub initial_checkpoint: Option<UtxoId>,
		#[allow(missing_docs)]
		pub _marker: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { main_chain_scripts: None, initial_checkpoint: None, _marker: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			MainChainScriptsConfiguration::<T>::set(self.main_chain_scripts.clone());
			DataCheckpoint::<T>::set(self.initial_checkpoint.map(BridgeDataCheckpoint::Utxo));
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Inherent extrinsic that handles all incoming transfers in the current block
		#[pallet::call_index(0)]
		#[pallet::weight((T::WeightInfo::handle_transfers(transfers.len() as u32), DispatchClass::Mandatory))]
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

		/// Changes the main chain scripts used for observing native token transfers along with a new data checkpoint.
		///
		/// This extrinsic must be run either using `sudo` or some other chain governance mechanism.
		///
		///
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_main_chain_scripts())]
		pub fn set_main_chain_scripts(
			origin: OriginFor<T>,
			new_scripts: MainChainScripts,
			data_checkpoint: BridgeDataCheckpoint,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			MainChainScriptsConfiguration::<T>::put(new_scripts);
			DataCheckpoint::<T>::put(data_checkpoint);
			Ok(())
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let data = Self::decode_inherent_data(data)?;
			let transfers = data.transfers.try_into().expect(
				"The number of transfers in the inherent data must be within configured bounds",
			);
			Some(Call::handle_transfers { transfers, data_checkpoint: data.data_checkpoint })
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let Some(expected_call) = Self::create_inherent(data) else {
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
