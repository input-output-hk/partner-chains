use crate as pallet_session_keys_registration;

use frame_support::derive_impl;
use frame_support::traits::{OnFinalize, OnInitialize};
use sp_core::parameter_types;
use sp_runtime::key_types::DUMMY;
use sp_runtime::testing::UintAuthorityId;
use sp_runtime::{BuildStorage, KeyTypeId};
use sp_staking::SessionIndex;

type Block = frame_system::mocking::MockBlock<Test>;

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub foo: UintAuthorityId,
	}
}

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Session: pallet_session,
		SessionKeysRegistration: pallet_session_keys_registration,
	}
);

type AccountId = u64;

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [KeyTypeId] = &[DUMMY];

	fn on_genesis_session<Ks: sp_runtime::traits::OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

	fn on_new_session<Ks: sp_runtime::traits::OpaqueKeys>(
		_: bool,
		_: &[(AccountId, Ks)],
		_: &[(AccountId, Ks)],
	) {
	}

	fn on_disabled(_: u32) {}
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

parameter_types! {
	pub const Period: u64 = 1;
	pub const Offset: u64 = 0;
}

impl pallet_session::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = u64;
	type ValidatorIdOf = sp_runtime::traits::ConvertInto;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = ();
	type SessionHandler = TestSessionHandler;
	type Keys = SessionKeys;
	type DisablingStrategy = ();
	type WeightInfo = ();
}

impl pallet_session_keys_registration::Config for Test {
	type PalletsOrigin = OriginCaller;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	sp_tracing::try_init_simple();
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	sp_io::TestExternalities::new(t)
}

pub fn start_session(session_index: SessionIndex) {
	for i in Session::current_index()..session_index {
		System::on_finalize(System::block_number());
		Session::on_finalize(System::block_number());

		let parent_hash = if System::block_number() > 1 {
			let hdr = System::finalize();
			hdr.hash()
		} else {
			System::parent_hash()
		};

		System::reset_events();
		System::initialize(&(i as u64 + 1), &parent_hash, &Default::default());
		System::set_block_number((i + 1).into());

		System::on_initialize(System::block_number());
		Session::on_initialize(System::block_number());
	}

	assert_eq!(Session::current_index(), session_index);
}
