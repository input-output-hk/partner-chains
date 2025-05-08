//!  Pallet for setting the Partner Chain validators using inherent data

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity)]
#![deny(missing_docs)]

pub mod migrations;
/// [`pallet_session`] integration.
#[cfg(feature = "pallet-session-compat")]
pub mod pallet_session_support;
#[cfg(feature = "pallet-session-compat")]
pub mod session_manager;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

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
	use sp_std::fmt::Display;
	use sp_std::{ops::Add, vec, vec::Vec};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The ubiquitous event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
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

	#[pallet::storage]
	pub type MainChainScriptsConfiguration<T: Config> =
		StorageValue<_, MainChainScripts, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// [Pallet::set] has been called with epoch number that is not current epoch + 1
		InvalidEpoch,
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
			let committee_info =
				CommitteeInfo { epoch: T::ScEpochNumber::zero(), committee: initial_authorities };
			CurrentCommittee::<T>::put(committee_info);
			MainChainScriptsConfiguration::<T>::put(self.main_chain_scripts.clone());
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		// Only reason for this hook is to set the genesis committee as the committee for first block's epoch.
		// If it wouldn't be set, the should_end_session() function would return true at the 2nd block,
		// thus denying handover phase to genesis committee, which would break the chain. With this hook,
		// should_end_session() returns true at 1st block and changes committee to the same one, thus allowing
		// handover phase to happen. After having proper chain initialization procedure this probably won't be needed anymore.
		// Note: If chain is started during handover phase, it will wait until new epoch to produce the first block.
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

		/// Responsible for calling [Call:set] on each block by the block author, if the validator list changed
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

		// TODO make this call run by every full node, so it can be relied upon for ensuring that the block is correct
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
		#[pallet::weight(T::WeightInfo::set(1))]
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
