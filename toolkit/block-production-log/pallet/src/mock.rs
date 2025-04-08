use crate::mock::sp_runtime::testing::H256;
use frame_support::sp_runtime::{
	BuildStorage,
	traits::{BlakeTwo256, IdentityLookup},
};
use frame_support::traits::{ConstU16, ConstU32, ConstU64};
use frame_support::*;
use sp_consensus_slots::Slot;

type AccountId = u32;
type Block = frame_system::mocking::MockBlock<Test>;

pub const MAX_LOG_SIZE: u32 = 100000;

#[frame_support::pallet]
pub mod mock_pallet {
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}
}

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		BlockProductionLog: crate::pallet,
		Mock: mock_pallet,
	}
);

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
	type Nonce = u64;
	type Block = Block;
	type RuntimeTask = RuntimeTask;
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

#[cfg(feature = "runtime-benchmarks")]
pub struct PalletBlockProductionLogBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl crate::benchmarking::BenchmarkHelper<[u8; 32]> for PalletBlockProductionLogBenchmarkHelper {
	fn producer_id() -> [u8; 32] {
		Default::default()
	}
}

impl crate::pallet::Config for Test {
	type BlockProducerId = [u8; 32];
	type WeightInfo = ();

	fn current_slot() -> Slot {
		Slot::from(1001000)
	}

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = PalletBlockProductionLogBenchmarkHelper;
}

impl mock_pallet::Config for Test {}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	sp_io::TestExternalities::new(storage)
}
