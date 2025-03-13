use crate::mock::sp_runtime::testing::H256;
use crate::{DispatchResult, TokenTransferHandler};
use frame_support::sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use frame_support::traits::{ConstU16, ConstU32, ConstU64};
use frame_support::*;
use sidechain_domain::*;

use self::mock_pallet::LastTokenTransfer;

type AccountId = u64;
type Block = frame_system::mocking::MockBlock<Test>;

#[frame_support::pallet]
pub mod mock_pallet {
	use frame_support::pallet_prelude::*;
	use sidechain_domain::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	pub type LastTokenTransfer<T: Config> = StorageValue<_, NativeTokenAmount, OptionQuery>;
}

impl<T: mock_pallet::Config> TokenTransferHandler for mock_pallet::Pallet<T> {
	fn handle_token_transfer(token_amount: NativeTokenAmount) -> DispatchResult {
		LastTokenTransfer::<T>::put(token_amount);
		Ok(())
	}
}

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		NativeTokenManagement: crate::pallet,
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
	type RuntimeEvent = RuntimeEvent;
	type TokenTransferHandler = Mock;
	type WeightInfo = ();
}

impl mock_pallet::Config for Test {}

pub fn new_test_ext() -> sp_io::TestExternalities {
	RuntimeGenesisConfig { system: Default::default(), native_token_management: Default::default() }
		.build_storage()
		.unwrap()
		.into()
}
