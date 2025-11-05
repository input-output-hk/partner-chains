use frame_support::pallet_prelude::*;
use frame_support::traits::ConstU32;
use frame_support::{
	construct_runtime,
	traits::{ConstU16, ConstU64},
};
use sp_core::H256;
use sp_runtime::{
	AccountId32, BuildStorage,
	traits::{BlakeTwo256, IdentityLookup},
};

use crate::BlockParticipationProvider;

pub type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = AccountId32;

#[frame_support::pallet]
pub mod mock_pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	pub type ProcessingTime<T: Config> = StorageMap<_, Twox64Concat, u64, u64, OptionQuery>;

	#[pallet::storage]
	pub type BlockProductionLog<T: Config> =
		StorageValue<_, BoundedVec<(u64, BlockProducerId), ConstU32<100>>>;
}

construct_runtime! {
	pub enum Test {
		System: frame_system,
		Payouts: crate::pallet,
		Mock: crate::mock::mock_pallet
	}
}

impl BlockParticipationProvider<u64, BlockProducerId> for Mock {
	fn blocks_to_process(up_to_moment: &u64) -> impl Iterator<Item = (u64, BlockProducerId)> {
		mock_pallet::BlockProductionLog::<Test>::get()
			.unwrap()
			.clone()
			.into_iter()
			.filter(move |(moment, _)| moment < up_to_moment)
	}

	fn discard_processed_blocks(up_to_moment: &u64) {
		let log = mock_pallet::BlockProductionLog::<Test>::get();
		if let Some(log) = log {
			let log = log.iter().filter(|(moment, _)| moment > up_to_moment).cloned().collect();
			mock_pallet::BlockProductionLog::<Test>::put(BoundedVec::truncate_from(log));
		}
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

const TEST_INHERENT_ID: InherentIdentifier = [42; 8];

impl crate::pallet::Config for Test {
	type WeightInfo = ();

	type BlockAuthor = BlockProducerId;
	type DelegatorId = DelegatorId;
	type Moment = u64;

	type BlockParticipationProvider = Mock;

	const TARGET_INHERENT_ID: InherentIdentifier = TEST_INHERENT_ID;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
