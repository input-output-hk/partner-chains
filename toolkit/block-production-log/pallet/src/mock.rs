use crate::mock::sp_runtime::testing::H256;
use frame_support::sp_runtime::{
	BuildStorage,
	traits::{BlakeTwo256, IdentityLookup},
};
use frame_support::traits::{ConstU16, ConstU32, ConstU64};
use frame_support::*;

type AccountId = u32;
type Block = frame_system::mocking::MockBlock<Test>;

pub(crate) type Moment = u64;
pub(crate) type BlockProducerId = [u8; 32];

#[frame_support::pallet]
pub mod mock_pallet {
	use crate::mock::{BlockProducerId, Moment};
	use frame_support::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	pub type BlockAuthor<T: Config> = StorageValue<_, BlockProducerId, ValueQuery>;

	#[pallet::storage]
	pub type CurrentMoment<T: Config> = StorageValue<_, Moment, ValueQuery>;

	impl<T: Config + crate::Config> crate::GetAuthor<BlockProducerId> for Pallet<T> {
		fn get_author() -> Option<BlockProducerId> {
			Some(BlockAuthor::<T>::get())
		}
	}

	impl<T: Config> crate::GetMoment<Moment> for Pallet<T> {
		fn get_moment() -> Option<Moment> {
			Some(CurrentMoment::<T>::get())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn set_block_author(author: BlockProducerId) {
			BlockAuthor::<T>::set(author)
		}
		pub fn set_moment(moment: Moment) {
			CurrentMoment::<T>::set(moment)
		}
	}
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

impl crate::pallet::Config for Test {
	type BlockProducerId = BlockProducerId;
	type Moment = Moment;
	type GetAuthor = Mock;
	type GetMoment = Mock;
}

impl mock_pallet::Config for Test {}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	sp_io::TestExternalities::new(storage)
}
