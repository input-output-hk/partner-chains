use frame_support::traits::ConstU32;
use frame_support::{
	construct_runtime,
	traits::{ConstU16, ConstU64},
};
use hex_literal::hex;
use scale_info::TypeInfo;
use sidechain_domain::byte_string::SizedByteString;
use sidechain_domain::*;
use sp_core::H256;
use sp_runtime::codec::{Decode, Encode, MaxEncodedLen};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, BoundedVec, BuildStorage,
};

pub type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = AccountId32;
pub type PartnerChainAddress = AccountId32;

construct_runtime! {
	pub enum Test {
		System: frame_system,
		BlockProducerMetadata: crate::pallet
	}
}

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

#[derive(Clone, Debug, MaxEncodedLen, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub struct BlockProducerUrlMetadata {
	pub url: BoundedVec<u8, ConstU32<512>>,
	pub hash: SizedByteString<32>,
}

impl crate::pallet::Config for Test {
	type WeightInfo = ();
	type BlockProducerMetadata = BlockProducerUrlMetadata;
	fn genesis_utxo() -> UtxoId {
		UtxoId::new(hex!("59104061ffa0d66f9ba0135d6fc6a884a395b10f8ae9cb276fc2c3bfdfedc260"), 1)
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
