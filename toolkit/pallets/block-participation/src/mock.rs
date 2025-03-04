use frame_support::pallet_prelude::*;
use frame_support::traits::ConstU32;
use frame_support::{
	construct_runtime,
	traits::{ConstU16, ConstU64},
};
use sp_block_participation::Slot;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, BuildStorage,
};

pub type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = AccountId32;

#[frame_support::pallet]
mod mock_pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	pub type CurrentSlot<T: Config> = StorageValue<_, Slot, ValueQuery>;

	#[pallet::storage]
	pub type SlotToPay<T: Config> = StorageMap<_, Twox64Concat, Slot, Slot, OptionQuery>;
}

construct_runtime! {
	pub enum Test {
		System: frame_system,
		ProductionLog: pallet_block_production_log,
		Payouts: crate::pallet,
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

type DelegatorId = u32;
type BlockProducerId = u64;

impl pallet_block_production_log::Config for Test {
	type BlockProducerId = BlockProducerId;

	type WeightInfo = ();

	fn current_slot() -> Slot {
		mock_pallet::CurrentSlot::<Test>::get()
	}
}

const TEST_INHERENT_ID: InherentIdentifier = [42; 8];

impl crate::pallet::Config for Test {
	type WeightInfo = ();

	type DelegatorId = DelegatorId;

	fn should_release_data(slot: Slot) -> Option<Slot> {
		mock_pallet::SlotToPay::<Test>::get(slot)
	}

	const TARGET_INHERENT_ID: InherentIdentifier = TEST_INHERENT_ID;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
