//! Pallet that establishes a Partner Chain as a Cardano sidechain
//!
//! # Purpose of this pallet
//!
//! This pallet serves as the starting point for building a Partner Chain runtime.
//! It stores its genesis UTXO which serves as its global identifier and divides
//! Partner Chain slots into Partner Chain epochs.
//!
//! ## Genesis UTXO
//!
//! When a Partner Chain governance is initialized on Cardano, the transaction spends
//! a special _genesis UTXO_. This UTXO serves multiple crucial roles:
//! - it serves as the unique identifier of the Partner Chain
//! - it affects the addresses of all Partner Chain Toolkit's Plutus smart contracts
//!   to allow multiple Partner Chains to exist
//! - it is included in messages signed by various participants of the Partner Chain
//!   when submitting transactions (on both Cardano and the Partner Chain) to prevent
//!   replay attacks
//!
//! The genesis UTXO is immutable throughout the lifetime of a Partner Chain in order
//! to preserve its identity.
//!
//! ## Partner Chain Epochs
//!
//! When producing blocks, Partner Chains divide time into slots in which a single block
//! can be produced. These slots are in turn grouped into epochs which other Partner
//! Chains features use as boundaries for some of their state transitions (eg. a Partner
//! Chain block producing committees change at epoch boundaries). Both slot and epoch
//! durations are constant and immutable throughout a Partner Chain's lifetime.
//!
//! # Usage
//!
//! ## Prerequisites
//!
//! Before a Partner Chain can be started, its Cardano governance must be established by
//! running the _init_ transaction, which also determines the _genesis UTXO_ of the
//! Partner Chain. Consult `docs/user-guides/governance/governance.md` for instructions.
//!
//! As Partner Chains operate on the basis of slots and epochs, your Substrate node should
//! use a slot-based consensus mechanism such as Aura.
//!
//! ### Optional - defining a new epoch hook
//!
//! A Partner Chain may need to perform custom logic when a new epoch starts, eg. pay
//! out block production rewards, update chain participant standings etc. For this purpose,
//! the pallet can be configured with a handler that will be triggered during initialization
//! of each first block of a new epoch.
//!
//! To create a new epoch handler, simply define a type and have it implement the [OnNewEpoch] trait:
//! ```rust
//! use sidechain_domain::{ ScEpochNumber, ScSlotNumber };
//! use sp_runtime::Weight;
//!
//! struct MyNewEpochHandler;
//! impl sp_sidechain::OnNewEpoch for MyNewEpochHandler {
//!     fn on_new_epoch(old_epoch: ScEpochNumber, new_epoch: ScEpochNumber) -> Weight {
//!         log::info!("Partner Chain epoch changed from {old_epoch} to {new_epoch}");
//!         Weight::zero()
//!     }
//! }
//! ```
//! The weight returned by `on_new_epoch` should match its real resource use.
//!
//! ## Adding to runtime
//!
//! The pallet requires minimal configuration, as it is only mandatory to inject the function
//! that provides the current slot. Assuming that Aura consensus is used, the pallet can
//! be configured like the following:
//! ```rust,ignore
//! impl pallet_sidechain::Config for Runtime {
//!     fn current_slot_number() -> ScSlotNumber {
//!         ScSlotNumber(*pallet_aura::CurrentSlot::<Self>::get())
//!     }
//!     type OnNewEpoch = MyNewEpochHandler;
//! }
//! ```
//! Optionally, a new epoch handler can be configured like in the example above. Partner Chains
//! that do not need to run additional logic at epoch change can use the empty implementation
//! available for [()]:
//! ```rust
//! # struct MyNewEpochHandler;
//! type OnNewEpoch = MyNewEpochHandler;
//! ```
//! If multiple handlers need to be added, a tuple can be used for convenience:
//! ```rust
//! # struct NewEpochHandler1;
//! # struct NewEpochHandler2;
//! type OnNewEpoch = (NewEpochHandler1, NewEpochHandler2);
//! ```
//!
//! ## Genesis configuration
//!
//! After the pallet is added to the runtime, configure it in your rust node code:
//! ```rust
//! # use std::str::FromStr;
//! # use sidechain_domain::UtxoId;
//! # use sidechain_slots::SlotsPerEpoch;
//! #
//! # fn create_genesis_config<Runtime>() -> pallet_sidechain::GenesisConfig<Runtime>
//! # where Runtime: frame_system::Config + pallet_sidechain::Config
//! # {
//! pallet_sidechain::GenesisConfig::<Runtime> {
//!     genesis_utxo: UtxoId::from_str("0000000000000000000000000000000000000000000000000000000000000000#0").unwrap(),
//!     slots_per_epoch: SlotsPerEpoch(60),
//!     ..Default::default()
//! }
//! # }
//! ```
//! or via a chain spec Json file:
//! ```json
//! {
//!     "sidechain_pallet": {
//!         "genesisUtxo": "0000000000000000000000000000000000000000000000000000000000000000#0",
//!         "slotsPerEpoch": 60
//!     }
//! }
//! ```
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

#[cfg(test)]
#[allow(missing_docs)]
pub mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::BlockNumberFor;
	use sidechain_domain::UtxoId;
	use sidechain_domain::{ScEpochNumber, ScSlotNumber};
	use sp_sidechain::OnNewEpoch;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Should return the slot number of the current block
		fn current_slot_number() -> ScSlotNumber;

		/// Handler that is called at initialization of the first block of a new Partner Chain epoch
		type OnNewEpoch: OnNewEpoch;
	}

	/// Current epoch number
	#[pallet::storage]
	pub(super) type EpochNumber<T: Config> = StorageValue<_, ScEpochNumber, ValueQuery>;

	/// Number of slots per epoch. Currently this value must not change for a running chain.
	#[pallet::storage]
	pub(super) type SlotsPerEpoch<T: Config> =
		StorageValue<_, sidechain_slots::SlotsPerEpoch, ValueQuery>;

	/// Genesis Cardano UTXO of the Partner Chain
	///
	/// This is the UTXO that is burned by the transaction that establishes Partner Chain
	/// governance on Cardano and serves as the identifier of the Partner Chain. It is also
	/// included in various signed messages to prevent replay attacks on other Partner Chains.
	#[pallet::storage]
	pub(super) type GenesisUtxo<T: Config> = StorageValue<_, UtxoId, ValueQuery>;

	impl<T: Config> Pallet<T> {
		/// Returns the genesis UTXO of the Partner Chain
		pub fn genesis_utxo() -> UtxoId {
			GenesisUtxo::<T>::get()
		}

		/// Returns current epoch number, based on slot number returned by [Config::current_slot_number]
		pub fn current_epoch_number() -> ScEpochNumber {
			let current_slot = T::current_slot_number();
			let slots_per_epoch = Self::slots_per_epoch();
			slots_per_epoch.epoch_number_from_sc_slot(current_slot)
		}

		/// Returns the configured number of slots per Partner Chain epoch
		pub fn slots_per_epoch() -> sidechain_slots::SlotsPerEpoch {
			SlotsPerEpoch::<T>::get()
		}
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// Genesis UTXO of the Partner Chain. This value is immutable.
		pub genesis_utxo: UtxoId,
		/// Number of slots ber Partner Chain epoch. This value is immutable.
		pub slots_per_epoch: sidechain_slots::SlotsPerEpoch,
		#[serde(skip)]
		#[allow(missing_docs)]
		pub _config: sp_std::marker::PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			GenesisUtxo::<T>::put(self.genesis_utxo);
			SlotsPerEpoch::<T>::put(self.slots_per_epoch);
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let real_epoch = Self::current_epoch_number();

			match EpochNumber::<T>::try_get().ok() {
				Some(saved_epoch) if saved_epoch != real_epoch => {
					log::info!("⏳ New epoch {real_epoch} starting at block {:?}", n);
					EpochNumber::<T>::put(real_epoch);
					<T::OnNewEpoch as OnNewEpoch>::on_new_epoch(saved_epoch, real_epoch)
						.saturating_add(T::DbWeight::get().reads_writes(2, 1))
				},
				None => {
					log::info!("⏳ Initial epoch {real_epoch} starting at block {:?}", n);
					EpochNumber::<T>::put(real_epoch);
					T::DbWeight::get().reads_writes(2, 1)
				},
				_ => T::DbWeight::get().reads_writes(2, 0),
			}
		}
	}
}
