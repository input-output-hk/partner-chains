use frame_support::{
	construct_runtime,
	pallet_prelude::*,
	traits::{ConstU16, ConstU32, ConstU64, Everything},
};
use frame_system::mocking::MockBlock;
use sp_consensus_slots::Slot;
use sp_io::TestExternalities;
use sp_runtime::{
	AccountId32, BuildStorage,
	traits::{BlakeTwo256, IdentityLookup},
};

#[frame_support::pallet]
pub mod mock_pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	pub type CurrentSlot<T: Config> = StorageValue<_, Slot, ValueQuery>;
}

construct_runtime! {
	pub enum Test {
		System: frame_system,
		BlockProducerFees: crate::pallet,
		Mock: crate::mock::mock_pallet
	}
}

impl mock_pallet::Config for Test {}

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Hash = sp_core::H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
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
	type Block = MockBlock<Test>;
	type Nonce = u64;
	type RuntimeTask = RuntimeTask;
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

#[cfg(feature = "runtime-benchmarks")]
pub struct PalletBlockProducerFeesBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl crate::benchmarking::BenchmarkHelper<AccountId32> for PalletBlockProducerFeesBenchmarkHelper {
	fn account_id(i: u8) -> AccountId32 {
		sp_core::sr25519::Public::from_raw([i; 32]).into()
	}
}

impl crate::pallet::Config for Test {
	type WeightInfo = ();

	// Stores the current and one historical value, two in total
	type HistoricalChangesPerProducer = ConstU16<1>;

	fn current_slot() -> Slot {
		mock_pallet::CurrentSlot::<Test>::get()
	}

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = PalletBlockProducerFeesBenchmarkHelper;
}

pub fn new_test_ext() -> TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
