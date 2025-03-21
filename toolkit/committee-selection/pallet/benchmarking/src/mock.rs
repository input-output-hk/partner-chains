use frame_support::{
	pallet_prelude::*,
	parameter_types,
	traits::{ConstBool, ConstU64},
};
use pallet_session_validator_management::pallet;
use serde::{Deserialize, Serialize};
use sidechain_domain::ScEpochNumber;
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = AccountId32;
type AuthorityId = AccountId32;

#[derive(
	PartialOrd,
	Ord,
	PartialEq,
	Eq,
	Debug,
	Clone,
	MaxEncodedLen,
	Encode,
	Decode,
	Serialize,
	Deserialize,
	TypeInfo,
)]
pub struct SessionKeys {
	pub aura: [u8; 32],
	pub grandpa: [u8; 32],
}

impl crate::Config for Test {}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Timestamp: pallet_timestamp,
		Aura: pallet_aura,
		SessionCommitteeManagement: pallet,
	}
);

parameter_types! {
	pub const SS58Prefix: u8 = 42;
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
	type SS58Prefix = SS58Prefix;
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

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

impl pallet_aura::Config for Test {
	type AuthorityId = sp_consensus_aura::ed25519::AuthorityId;
	type MaxAuthorities = ConstU32<32>;
	type DisabledValidators = ();
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type SlotDuration = ConstU64<6000>;
}

impl pallet::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MaxValidators = ConstU32<32>;
	type AuthorityId = AuthorityId;
	type AuthorityKeys = SessionKeys;
	type AuthoritySelectionInputs = ();
	type ScEpochNumber = ScEpochNumber;
	type CommitteeMember = (Self::AuthorityId, Self::AuthorityKeys);

	fn select_authorities(
		_: Self::AuthoritySelectionInputs,
		_: ScEpochNumber,
	) -> Option<BoundedVec<Self::CommitteeMember, Self::MaxValidators>> {
		todo!("not used in benchmarks")
	}

	fn current_epoch_number() -> ScEpochNumber {
		sidechain_slots::epoch_number(
			pallet_aura::CurrentSlot::<Test>::get(),
			crate::SLOTS_PER_EPOCH,
		)
	}

	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	sp_io::TestExternalities::new(t)
}
