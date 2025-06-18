//! Mock runtime for pallet-cardano-config tests

use crate as pallet_cardano_config;
use frame_support::{
	derive_impl, parameter_types,
	traits::{ConstU16, ConstU32, ConstU64},
};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		CardanoConfig: pallet_cardano_config,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = sp_core::H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_cardano_config::Config for Test {
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}

// Build genesis storage with CardanoConfig preset
pub fn new_test_ext_with_config() -> sp_io::TestExternalities {
	use sidechain_domain::{CardanoConfig, MainchainEpochConfig};
	use sp_core::offchain::{Duration, Timestamp};

	let config = CardanoConfig {
		epoch_config: MainchainEpochConfig {
			epoch_duration_millis: Duration::from_millis(432000000), // 5 days
			slot_duration_millis: Duration::from_millis(1000),       // 1 second
			first_epoch_timestamp_millis: Timestamp::from_unix_millis(1596059091000),
			first_epoch_number: 208,
			first_slot_number: 4492800,
		},
		cardano_security_parameter: 432,
		cardano_active_slots_coeff: 0.05,
	};

	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_cardano_config::GenesisConfig::<Test> {
		cardano_config: Some(config),
		_marker: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}
