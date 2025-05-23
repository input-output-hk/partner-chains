use frame_support::traits::ConstU32;
use frame_support::{
	construct_runtime,
	traits::{ConstU16, ConstU64},
};
use frame_system::EnsureRoot;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sidechain_domain::byte_string::BoundedString;
use sp_core::H256;
use sp_governed_map::OnGovernedMappingChange;
use sp_runtime::{
	AccountId32, BoundedVec, BuildStorage,
	traits::{BlakeTwo256, IdentityLookup},
};

pub type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = AccountId32;

pub(crate) const TEST_MAX_CHANGES: u32 = 8;
pub(crate) type MaxChanges = ConstU32<TEST_MAX_CHANGES>;
pub(crate) type MaxKeyLength = ConstU32<64>;
pub(crate) type MaxValueLength = ConstU32<512>;

#[derive(Debug, Decode, Encode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, PartialEq, Eq)]
pub enum MappingChange {
	Created {
		key: BoundedString<MaxKeyLength>,
		value: BoundedVec<u8, MaxValueLength>,
	},
	Updated {
		key: BoundedString<MaxKeyLength>,
		old_value: BoundedVec<u8, MaxValueLength>,
		new_value: BoundedVec<u8, MaxValueLength>,
	},
	Deleted {
		key: BoundedString<MaxKeyLength>,
		old_value: BoundedVec<u8, MaxValueLength>,
	},
}

#[frame_support::pallet]
pub mod mock_pallet {
	use frame_support::pallet_prelude::*;

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	#[pallet::unbounded]
	pub type HookCalls<T: Config> = StorageValue<_, Vec<MappingChange>, ValueQuery>;

	impl<T: Config> OnGovernedMappingChange<MaxKeyLength, MaxValueLength> for Pallet<T> {
		fn on_governed_mapping_change(
			key: BoundedString<MaxKeyLength>,
			new_value: Option<BoundedVec<u8, MaxValueLength>>,
			old_value: Option<BoundedVec<u8, MaxValueLength>>,
		) {
			let change = match (old_value, new_value) {
				(None, Some(value)) => MappingChange::Created { key, value },
				(Some(old_value), Some(new_value)) => {
					MappingChange::Updated { key, old_value, new_value }
				},
				(Some(old_value), None) => MappingChange::Deleted { key, old_value },
				_ => unreachable!(),
			};

			HookCalls::<T>::append(change);
		}
	}
}

construct_runtime! {
	pub enum Test {
		System: frame_system,
		GovernedMap: crate::pallet,
		Mock: crate::mock::mock_pallet
	}
}

impl mock_pallet::Config for Test {}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type ExtensionsWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type Block = Block;
	type Nonce = u64;
	type RuntimeTask = RuntimeTask;
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

impl crate::pallet::Config for Test {
	type MaxChanges = MaxChanges;

	type MaxKeyLength = MaxKeyLength;

	type MaxValueLength = MaxValueLength;

	type OnGovernedMappingChange = Mock;

	type MainChainScriptsOrigin = EnsureRoot<Self::AccountId>;

	type WeightInfo = ();

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}

/// Current mapping state ordered by key
pub fn mappings_in_storage() -> Vec<(BoundedString<MaxKeyLength>, BoundedVec<u8, MaxValueLength>)> {
	let mut storage_state = GovernedMap::get_all_key_value_pairs().collect::<Vec<_>>();
	storage_state.sort_by_key(|m| m.0.to_string());
	storage_state
}
