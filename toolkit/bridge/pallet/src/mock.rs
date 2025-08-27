use frame_support::traits::ConstU32;
use frame_support::{
	construct_runtime,
	traits::{ConstU16, ConstU64},
};
use frame_system::EnsureRoot;
use sidechain_domain::byte_string::BoundedString;
use sp_core::H256;
use sp_partner_chains_bridge::BridgeTransferV1;
use sp_runtime::{
	AccountId32, BuildStorage,
	traits::{BlakeTwo256, IdentityLookup},
};

pub type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = AccountId32;
pub type RecipientAddress = BoundedString<ConstU32<64>>;
pub type MaxTransfersPerBlock = ConstU32<32>;

#[frame_support::pallet]
pub mod mock_pallet {
	use frame_support::pallet_prelude::*;

	use crate::TransferHandler;

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	#[pallet::unbounded]
	pub type Transfers<T: Config> = StorageValue<_, Vec<BridgeTransferV1<RecipientAddress>>>;

	impl<T> TransferHandler<RecipientAddress> for Pallet<T> {
		fn handle_incoming_transfer(transfer: BridgeTransferV1<RecipientAddress>) {
			Transfers::<Test>::append(transfer);
		}
	}
}

construct_runtime! {
	pub enum Test {
		System: frame_system,
		Bridge: crate::pallet,
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

impl crate::Config for Test {
	type GovernanceOrigin = EnsureRoot<AccountId>;
	type Recipient = RecipientAddress;
	type TransferHandler = Mock;
	type MaxTransfersPerBlock = MaxTransfersPerBlock;
	type WeightInfo = ();

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
