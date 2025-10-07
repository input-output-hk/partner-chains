//! Pallet for setting the Partner Chain validators using inherent data
//!
//! # Purpose of the pallet
//!
//! This pallet provides a mechanism to rotate Partner Chain's block producing committees
//! based on candidate registrations and chain configuration sourced from Cardano. It works
//! by integrating with stock Substrate `pallet_session` as a [SessionManager] to provide it
//! with committee information and to rotate sessions. In addition to managing the sessions,
//! the pallet automatically registers session keys of all active block producers, alleviating
//! the need for manual key registration, and ensures that all necessary chain-local accounts
//! exist.
//!
//! # Committee selection overview
//!
//! Committees are selected for sessions corresponding roughly to Partner Chain epochs, whose
//! duration is configurable for each Partner Chain. Due to the way session rotation works in
//! `pallet_session`, these sessions are delayed by *2 blocks* relative to their respective
//! epoch.
//!
//! Committees are selected based on the following inputs sourced from Cardano:
//! - `Registered candidates`:
//!   Cardano SPOs who have registered themselves as willing to participate as block producers.
//!   These candidates need to control an ADA stake pool to be eligible for selection to a
//!   committee, and their chance at securing a seat is proportional to their pool's size.
//!   This candidate group corresponds to a typical "trustless" Proof of Stake block producers.
//! - `Permissioned candidates`:
//!   A list of trusted block producers that do not need to register themselves or control any
//!   ADA stake on Cardano to be eligible for a Partner Chain committee.
//!   This candidate group serves a special role as trusted block producers during initial phase
//!   of a Partner Chain's lifetime (when there may not be enough registered candidates to ensure
//!   proper security and decentralization of the network), and are intended to be phased out as
//!   the number of trustless participants grows.
//! - `D-Parameter`:
//!   A pair of two values `R` and `P`, controlling the number of committee seats alloted for
//!   registered and permissioned candidates respectively, which means that a committee has R+P
//!   seats overall. This parameter gives the Partner Chain the ability to bootstrap itself using
//!   an initial pool of permissioned candidates running trusted nodes, and then gradually shift
//!   to registered (trustless) candidates when proper decentralization is achieved
//! - `randomness seed`:
//!   All randomness when selecting the committee is seeded from data sourced from Cardano so that
//!   it is tamper-proof and agreed upon by all nodes.
//!
//! The permissioned candidate list and the D-Parameter are controlled by the Partner Chain's
//! governance authority and are crucial in ensuring the chain's security in initial phases of
//! its existence
//!
//! # Observability parameters
//!
//! All input data used when selecting a committee of a Partner Chain is sourced from Cardano.
//! To correctly identify it, each node needs access to the current values of:
//! - `the registration validator address`, at which all registration UTXOs are located
//! - `the D-Parameter minting policy`, whose tokens mark the UTXO containing D-Parameter value
//! - `the permissioned candidate minting policy`, whose tokens mark the UTXO containing the
//!   permissioned candidate list
//!
//! These values are stored in the pallet storage, ensuring that they're available for all nodes
//! to use and agreed upon through the consensus mechanism, and can be updated using a governance
//! level extrinsic [set_main_chain_scripts].
//!
//! # Usage
//!
//! ## Prerequisites
//!
//! This pallet's operation requires the appropriate inherent data provider and data source
//! be present in the node. As this pallet is crucial for the operation of the chain itself,
//! these must be present before at the chain start or before the pallet is migrated to, to
//! avoid down time. See documentation of `sp_session_validator_management` for information
//! on how to add the IDP to your node. A Db-Sync-based data source implementation is provided
//! by the `partner_chains_db_sync_data_sources` crate.
//!
//! Aside from the node components, the pallet requires the Partner Chain smart contracts to
//! have been initialized on Cardano and that at least one candidate - either a registered or
//! permissioned one - exists. See `docs/user-guides/governance/governance.md` and
//! `docs/user-guides/chain-builder.md` for more information about governance and how to set
//! up the Partner Chain on Cardano.
//!
//! ## Adding into the runtime
//!
//! ### Defining key types
//!
//! As with a stock Substrate chain, a Partner Chain needs to define its session keys. What
//! these keys are depends on the consensus mechanisms used by the chain. For a Partner Chain
//! using Aura as its consensus with a Grandpa finality gadget, the session keys can be defined
//! as following:
//!
//! ```rust, ignore
//! sp_runtime::impl_opaque_keys! {
//! 	#[derive(MaxEncodedLen, PartialOrd, Ord)]
//! 	pub struct SessionKeys {
//! 		pub aura: Aura
//! 		pub grandpa: Grandpa,
//! 	}
//! }
//! ```
//!
//! In addition to the session keys, the runtime needs to define an ECDSA key type to represent
//! the `cross-chain key`:
//! ```rust
//! pub mod cross_chain_app {
//!     use sp_runtime::KeyTypeId;
//!     use sp_runtime::app_crypto::{ app_crypto, ecdsa };
//!     pub const CROSS_CHAIN: KeyTypeId = KeyTypeId(*b"crch");
//! 	app_crypto!(ecdsa, CROSS_CHAIN);
//! }
//! pub type CrossChainPublic = cross_chain_app::Public;
//! ```
//!
//! This key serves as the identity of a Partner Chain user across all chains in the ecosystem.
//!
//! ### Adding the pallet
//!
//! The pallet should be added to the runtime _before_ `pallet_session`, but after the consensus
//! pallets used by the chain:
//!
//! ```rust, ignore
//! construct_runtime!(
//! 	pub struct Runtime {
//! 		System: frame_system,
//! 		Timestamp: pallet_timestamp,
//! 		Aura: pallet_aura,
//! 		Grandpa: pallet_grandpa,
//! 		Sidechain: pallet_sidechain,
//! 		SessionCommitteeManagement: pallet_session_validator_management,
//! 		Session: pallet_session exclude_parts { Call },
//!         // ... other pallets
//! 	}
//! );
//! ```
//!
//! *Important*:
//! It is recommended that when `pallet_session` is wired into the runtime, its extrinsics are
//! hidden, using `exclude_parts` like in the example above. This ensures that chain users can't
//! manually register their keys in the pallet and so the registrations done on Cardano remain
//! the sole source of truth about key ownership. Proper operation in presence of manually set
//! user keys is not guaranteed by the toolkit and its behavior is left unspecified.
//!
//! ### Configuring the pallet
//!
//! Configuring the pallet is straightforward and mostly consists of passing to it types already
//! defined by other crates and in previous steps:
//!
//! ```rust, ignore
//! impl pallet_session_validator_management::Config for Runtime {
//! 	type MaxValidators = MaxValidators;
//! 	type AuthorityId = CrossChainPublic;
//! 	type AuthorityKeys = SessionKeys;
//! 	type AuthoritySelectionInputs = authority_selection_inherents::AuthoritySelectionInputs;
//! 	type ScEpochNumber = sidechain_domain::ScEpochNumber;
//! 	type WeightInfo = pallet_session_validator_management::weights::SubstrateWeight<Runtime>;
//! 	type CommitteeMember = authority_selection_inherents::CommitteeMember<CrossChainPublic, SessionKeys>;
//! 	type MainChainScriptsOrigin = EnsureRoot<Self::AccountId>;
//!
//! 	fn select_authorities(
//! 		input: AuthoritySelectionInputs,
//! 		sidechain_epoch: ScEpochNumber,
//! 	) -> Option<BoundedVec<Self::CommitteeMember, Self::MaxValidators>> {
//! 		authority_selection_inherents::select_authorities::<CrossChainPublic, SessionKeys, MaxValidators>(
//! 			Sidechain::genesis_utxo(),
//! 			input,
//! 			sidechain_epoch,
//! 		)
//! 	}
//!
//! 	fn current_epoch_number() -> ScEpochNumber {
//! 		Sidechain::current_epoch_number()
//! 	}
//! }
//! ```
//!
//! One value that needs to be decided upon by the chain builder is `MaxValidators` which dictates
//! the maximum size of a committee. This value should be higher than the P + R of the D-Parameter
//! used and should be adjusted accordingly before any D-Parameter changes that would exceed the
//! previous value. In case a committee selected is bigger than `MaxValidators`, it will be truncated,
//! potentially leading to a skewed seat allocation and threatening the security of the consensus.
//!
//! ## Genesis configuration
//!
//! Genesis config can be created programmatically:
//!
//! ```rust,ignore
//! GenesisConfig {
//! 	initial_authorities: vec![
//!        CommitteeMember::permissioned(cross_chain_pubkey_1, session_keys_1),
//!     ],
//! 	main_chain_scripts: MainChainScripts::read_from_env()?,
//! }
//! ```
//!
//! However, it is more typical for production chains to define their specs using Json. In that case
//! an example configuration could look like this:
//!
//! ```json
//! {
//!   "initialAuthorities": [
//!     {
//!       "Permissioned": {
//!         "id": "KW39r9CJjAVzmkf9zQ4YDb2hqfAVGdRqn53eRqyruqpxAP5YL",
//!         "keys": {
//!           "aura": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
//!           "grandpa": "5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu"
//!         }
//!       }
//!     }
//!   ],
//!   "mainChainScripts": {
//!     "committee_candidate_address": "addr_test1wrp8p2c5h7encl55gv26d5fpz9r99jxjcm0rxgny3993dxs2xy42p",
//!     "d_parameter_policy_id": "0x434dc797fd036b0b654c005551ec08f39d25fa7f0eecdf4b170d46cf",
//!     "permissioned_candidates_policy_id": "0xe1ce5d1b8b3e93a7493ecc11556790f915aabbc44a56b0b5145770b2"
//!   }
//! }
//! ```
//!
//! *Important*:
//! Notice, that since the pallet's operation is necessary for block production, all main chain script
//! values and at least one initial authority (block producer) must be provided by the genesis config.
//!
//!
//! ## Updating pallet configuration
//!
//! ### MaxValidators
//!
//! The maximum number of committee seats. As this value is not typically expected to change, it is
//! configured as part of the pallet's [Config]. This means that it can only be updated as part of a
//! runtime upgrade. The chain builder should release a new runtime version with this value updated
//! and the Partner Chain's governance mechanism should be used to apply it using [set_code].
//!
//! ### Main chain scripts
//!
//! The main chain scripts can change over time as the Partner Chain migrates to new versions of the
//! Patner Chain smart contracts, either due to bug fixes or new features being released. This is
//! necessary, because the script addresses are derived by hashing their Plutus code and are affected
//! by any change made to it.
//!
//! The scripts are updated by invoking the [set_main_chain_scripts] extrinsic using the Partner Chain's
//! governance mechanism.
//!
//! *Important*: Setting incorrect main chain script values will result in stalling block production
//!              indefinitely, requiring a network-wide roll-back. As such, main chain scripts update
//!              should be carried out with special care.
//!
//! [SessionManager]: pallet_session::SessionManager
//! [set_code]: frame_system::Pallet::set_code

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity)]
#![deny(missing_docs)]

pub mod migrations;
/// [`pallet_session`] integration.
pub mod pallet_session_support;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod tests;

pub mod weights;

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
pub use sp_session_validator_management::CommitteeMember;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use log::{info, warn};
	use sidechain_domain::byte_string::SizedByteString;
	use sidechain_domain::{MainchainAddress, PolicyId};
	use sp_core::blake2_256;
	use sp_runtime::traits::{MaybeSerializeDeserialize, One, Zero};
	use sp_session_validator_management::*;
	use sp_std::collections::btree_set::BTreeSet;
	use sp_std::fmt::Display;
	use sp_std::{ops::Add, vec::Vec};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		#[pallet::constant]
		/// Maximum amount of validators.
		type MaxValidators: Get<u32>;
		/// Type identifying authorities.
		type AuthorityId: Member
			+ Parameter
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ Ord
			+ Into<Self::AccountId>;
		/// Type of authority keys.
		type AuthorityKeys: Parameter + Member + MaybeSerializeDeserialize + Ord + MaxEncodedLen;
		/// Type of input data used by `select_authorities` to select a committee.
		type AuthoritySelectionInputs: Parameter;
		/// Type of the epoch number used by the partner chain.
		type ScEpochNumber: Parameter
			+ MaxEncodedLen
			+ Zero
			+ Display
			+ Add
			+ One
			+ Default
			+ Ord
			+ Copy
			+ From<u64>
			+ Into<u64>;

		/// Type of committee members returned by `select_authorities`.
		type CommitteeMember: Parameter
			+ Member
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ CommitteeMember<AuthorityId = Self::AuthorityId, AuthorityKeys = Self::AuthorityKeys>;

		/// Origin for governance calls
		type MainChainScriptsOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Should select a committee for `sidechain_epoch` based on selection inputs `input`.
		/// Should return [None] if selection was impossible for the given input.
		fn select_authorities(
			input: Self::AuthoritySelectionInputs,
			sidechain_epoch: Self::ScEpochNumber,
		) -> Option<BoundedVec<Self::CommitteeMember, Self::MaxValidators>>;

		/// Should return the current partner chain epoch.
		fn current_epoch_number() -> Self::ScEpochNumber;

		/// Weight functions needed for pallet_session_validator_management.
		type WeightInfo: WeightInfo;

		/// Helper for creating mock data used by benchmarks
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: benchmarking::BenchmarkHelper<Self>;
	}

	#[pallet::event]
	pub enum Event<T: Config> {}

	use frame_support::{BoundedVec, CloneNoBound};
	use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
	use scale_info::TypeInfo;

	#[derive(CloneNoBound, Encode, Decode, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(MaxValidators))]
	/// Committee info type used on-chain.
	pub struct CommitteeInfo<ScEpochNumber: Clone, CommitteeMember: Clone, MaxValidators> {
		/// Epoch number the committee is selected for.
		pub epoch: ScEpochNumber,
		/// List of committee members.
		pub committee: BoundedVec<CommitteeMember, MaxValidators>,
	}

	impl<ScEpochNumber: Clone, CommitteeMember: Clone, MaxValidators>
		CommitteeInfo<ScEpochNumber, CommitteeMember, MaxValidators>
	{
		/// Returns committee info as a pair of epoch number and list of committee members
		pub fn as_pair(self) -> (ScEpochNumber, Vec<CommitteeMember>) {
			(self.epoch, self.committee.to_vec())
		}
	}

	impl<ScEpochNumber, CommitteeMember, MaxValidators> Default
		for CommitteeInfo<ScEpochNumber, CommitteeMember, MaxValidators>
	where
		CommitteeMember: Clone,
		ScEpochNumber: Clone + Zero,
	{
		fn default() -> Self {
			Self { epoch: ScEpochNumber::zero(), committee: BoundedVec::new() }
		}
	}

	#[pallet::storage]
	pub type ProvidedAccounts<T: Config> =
		StorageValue<_, BoundedBTreeSet<T::AccountId, T::MaxValidators>, ValueQuery>;

	#[pallet::storage]
	pub type CurrentCommittee<T: Config> = StorageValue<
		_,
		CommitteeInfo<T::ScEpochNumber, T::CommitteeMember, T::MaxValidators>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type NextCommittee<T: Config> = StorageValue<
		_,
		CommitteeInfo<T::ScEpochNumber, T::CommitteeMember, T::MaxValidators>,
		OptionQuery,
	>;

	/// Stores the stage of handling the inputs change. Used by session manager, to decide
	/// if the session should be ended quickly, to speed up using the newly selected committee.
	#[pallet::storage]
	pub type CommitteeRotationStage<T: Config> =
		StorageValue<_, CommitteeRotationStages, ValueQuery>;

	#[pallet::storage]
	pub type MainChainScriptsConfiguration<T: Config> =
		StorageValue<_, MainChainScripts, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// [Pallet::set] has been called with epoch number that is not current epoch + 1
		InvalidEpoch,
		/// [Pallet::set] has been called a second time for the same next epoch
		NextCommitteeAlreadySet,
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// Initial committee members of the partner chain.
		pub initial_authorities: Vec<T::CommitteeMember>,
		/// Initial [MainChainScripts] of the partner chain.
		pub main_chain_scripts: MainChainScripts,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			let initial_authorities = BoundedVec::truncate_from(self.initial_authorities.clone());

			let provided_accounts: BTreeSet<T::AccountId> =
				initial_authorities.iter().map(|m| m.authority_id().into()).collect();
			for account in &provided_accounts {
				frame_system::Pallet::<T>::inc_providers(&account);
			}
			ProvidedAccounts::<T>::set(provided_accounts.try_into().unwrap());

			let committee_info =
				CommitteeInfo { epoch: T::ScEpochNumber::zero(), committee: initial_authorities };
			CurrentCommittee::<T>::put(committee_info);
			MainChainScriptsConfiguration::<T>::put(self.main_chain_scripts.clone());
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		// Only reason for this hook is to set the genesis committee as the committee for first block's epoch.
		fn on_initialize(block_nr: BlockNumberFor<T>) -> Weight {
			if block_nr.is_one() {
				CurrentCommittee::<T>::mutate(|committee| {
					committee.epoch = T::current_epoch_number();
				});
				T::DbWeight::get().reads_writes(2, 1)
			} else {
				Weight::zero()
			}
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		/// Responsible for calling [Call::set] on each block by the block author, if the validator list changed
		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			if NextCommittee::<T>::exists() {
				None
			} else {
				let for_epoch_number = CurrentCommittee::<T>::get().epoch + One::one();
				let (authority_selection_inputs, selection_inputs_hash) =
					Self::inherent_data_to_authority_selection_inputs(data);
				if let Some(validators) =
					T::select_authorities(authority_selection_inputs, for_epoch_number)
				{
					Some(Call::set { validators, for_epoch_number, selection_inputs_hash })
				} else {
					let current_committee = CurrentCommittee::<T>::get();
					let current_committee_epoch = current_committee.epoch;
					warn!(
						"Committee for epoch {for_epoch_number} is the same as for epoch {current_committee_epoch}"
					);
					let validators = current_committee.committee;
					Some(Call::set { validators, for_epoch_number, selection_inputs_hash })
				}
			}
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let (validators_param, for_epoch_number_param, call_selection_inputs_hash) = match call
			{
				Call::set { validators, for_epoch_number, selection_inputs_hash } => {
					(validators, for_epoch_number, selection_inputs_hash)
				},
				_ => return Ok(()),
			};

			let (authority_selection_inputs, computed_selection_inputs_hash) =
				Self::inherent_data_to_authority_selection_inputs(data);
			let validators =
				T::select_authorities(authority_selection_inputs, *for_epoch_number_param)
					.unwrap_or_else(|| {
						// Proposed block should keep the same committee if calculation of new one was impossible.
						// This is code is executed before the committee rotation, so the NextCommittee should be used.
						let committee_info = NextCommittee::<T>::get()
							// Needed only for verification of the block no 1, before any `set` call is executed.
							.unwrap_or_else(CurrentCommittee::<T>::get);
						committee_info.committee
					});

			if *validators_param != validators {
				if *call_selection_inputs_hash == computed_selection_inputs_hash {
					return Err(InherentError::InvalidValidatorsMatchingHash(
						computed_selection_inputs_hash,
					));
				} else {
					return Err(InherentError::InvalidValidatorsHashMismatch(
						computed_selection_inputs_hash,
						call_selection_inputs_hash.clone(),
					));
				}
			}

			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::set { .. })
		}

		fn is_inherent_required(_: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			if !NextCommittee::<T>::exists() {
				Ok(Some(InherentError::CommitteeNeedsToBeStoredOneEpochInAdvance)) // change error
			} else {
				Ok(None)
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// 'for_epoch_number' parameter is needed only for validation purposes, because we need to make sure that
		/// check_inherent uses the same epoch_number as was used to create inherent data.
		/// Alternative approach would be to put epoch number inside InherentData. However, sidechain
		/// epoch number is set in Runtime, thus, inherent data provider doesn't have to know about it.
		/// On top of that, the latter approach is slightly more complicated to code.
		#[pallet::call_index(0)]
		#[pallet::weight((
		T::WeightInfo::set(validators.len() as u32),
		DispatchClass::Mandatory
		))]
		pub fn set(
			origin: OriginFor<T>,
			validators: BoundedVec<T::CommitteeMember, T::MaxValidators>,
			for_epoch_number: T::ScEpochNumber,
			selection_inputs_hash: SizedByteString<32>,
		) -> DispatchResult {
			ensure_none(origin)?;
			let expected_epoch_number = CurrentCommittee::<T>::get().epoch + One::one();
			ensure!(for_epoch_number == expected_epoch_number, Error::<T>::InvalidEpoch);
			ensure!(!NextCommittee::<T>::exists(), Error::<T>::NextCommitteeAlreadySet);
			let len = validators.len();
			info!(
				"💼 Storing committee of size {len} for epoch {for_epoch_number}, input data hash: {}",
				selection_inputs_hash.to_hex_string()
			);
			NextCommittee::<T>::put(CommitteeInfo {
				epoch: for_epoch_number,
				committee: validators,
			});
			Ok(())
		}

		/// Changes the main chain scripts used for committee rotation.
		///
		/// This extrinsic must be run either using `sudo` or some other chain governance mechanism.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_main_chain_scripts())]
		pub fn set_main_chain_scripts(
			origin: OriginFor<T>,
			committee_candidate_address: MainchainAddress,
			d_parameter_policy_id: PolicyId,
			permissioned_candidates_policy_id: PolicyId,
		) -> DispatchResult {
			T::MainChainScriptsOrigin::ensure_origin(origin)?;
			let new_scripts = MainChainScripts {
				committee_candidate_address,
				d_parameter_policy_id,
				permissioned_candidates_policy_id,
			};
			MainChainScriptsConfiguration::<T>::put(new_scripts);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns epoch number for which next committee hasn't been set yet.
		pub fn get_next_unset_epoch_number() -> T::ScEpochNumber {
			NextCommittee::<T>::get()
				.map(|next_committee| next_committee.epoch + One::one())
				.unwrap_or(CurrentCommittee::<T>::get().epoch + One::one())
		}

		/// Returns current committee member for an index, repeating them in a round-robin fashion if needed.
		pub fn get_current_authority_round_robin(index: usize) -> Option<T::CommitteeMember> {
			let committee = CurrentCommittee::<T>::get().committee;
			if committee.is_empty() {
				return None;
			}

			committee.get(index % committee.len() as usize).cloned()
		}

		/// Returns current committee from storage.
		pub fn current_committee_storage()
		-> CommitteeInfo<T::ScEpochNumber, T::CommitteeMember, T::MaxValidators> {
			CurrentCommittee::<T>::get()
		}

		/// Returns next committee from storage.
		pub fn next_committee_storage()
		-> Option<CommitteeInfo<T::ScEpochNumber, T::CommitteeMember, T::MaxValidators>> {
			NextCommittee::<T>::get()
		}

		/// Returns the `AuthorityId`s of next committee from storage.
		///
		/// This function's result should be always defined after inherent call of 1st block of each epoch
		pub fn next_committee() -> Option<BoundedVec<T::AuthorityId, T::MaxValidators>> {
			Some(BoundedVec::truncate_from(
				NextCommittee::<T>::get()?
					.committee
					.into_iter()
					.map(|member| member.authority_id())
					.collect::<Vec<T::AuthorityId>>(),
			))
		}

		fn inherent_data_to_authority_selection_inputs(
			data: &InherentData,
		) -> (T::AuthoritySelectionInputs, SizedByteString<32>) {
			let decoded_data = data
				.get_data::<T::AuthoritySelectionInputs>(&INHERENT_IDENTIFIER)
				.expect("Validator inherent data not correctly encoded")
				.expect("Validator inherent data must be provided");
			let data_hash = SizedByteString(blake2_256(&decoded_data.encode()));

			(decoded_data, data_hash)
		}

		/// Calculates committee using configured `select_authorities` function
		pub fn calculate_committee(
			authority_selection_inputs: T::AuthoritySelectionInputs,
			sidechain_epoch: T::ScEpochNumber,
		) -> Option<Vec<T::CommitteeMember>> {
			T::select_authorities(authority_selection_inputs, sidechain_epoch).map(|c| c.to_vec())
		}

		/// If [NextCommittee] is defined, it moves its value to [CurrentCommittee] storage.
		/// Returns the value taken from [NextCommittee].
		pub fn rotate_committee_to_next_epoch() -> Option<Vec<T::CommitteeMember>> {
			let next_committee = NextCommittee::<T>::take()?;

			CurrentCommittee::<T>::put(next_committee.clone());

			let validators = next_committee.committee.to_vec();
			let len = validators.len();
			info!(
				"Committee rotated: Returning {len} validators, stored in epoch {}",
				next_committee.epoch
			);
			Some(validators)
		}

		/// Returns main chain scripts.
		pub fn get_main_chain_scripts() -> MainChainScripts {
			MainChainScriptsConfiguration::<T>::get()
		}
	}
}

/// For session state machine
#[derive(Encode, Decode, Default, Debug, MaxEncodedLen, TypeInfo, PartialEq, Eq)]
pub enum CommitteeRotationStages {
	/// No action is required until the current committee becomes obsolete
	#[default]
	AwaitEpochChange,
	/// Session ended because of epoch change
	NewSessionDueEpochChange,
	/// Session ended to accelerate use of validators queued in the previous block
	AdditionalSession,
}
