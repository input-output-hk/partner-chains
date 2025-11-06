use crate::{Call, CommitteeMemberOf, pallet};
use frame_support::{
	derive_impl,
	dispatch::PostDispatchInfo,
	pallet_prelude::*,
	parameter_types,
	traits::{ConstU64, UnfilteredDispatchable},
};
use frame_system::EnsureRoot;
use pallet_balances::AccountData;
use sidechain_domain::{
	CandidateKey, DParameter, EpochNonce, PermissionedCandidateData, SidechainPublicKey,
	byte_string::SizedByteString,
};
use sidechain_domain::{CandidateKeys, ScEpochNumber};
use sp_core::{H256, blake2_256, ecdsa};
use sp_runtime::{
	AccountId32, BuildStorage, KeyTypeId,
	key_types::DUMMY,
	testing::UintAuthorityId,
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_session_validator_management::{
	AuthoritySelectionInputs, CommitteeMember, MainChainScripts,
};

type CrossChainPublic = sidechain_domain::cross_chain_app::Public;

type Block = frame_system::mocking::MockBlock<Test>;

pub(crate) type AccountId = AccountId32;
pub(crate) type AuthorityId = CrossChainPublic;

sp_runtime::impl_opaque_keys! {
	#[derive(MaxEncodedLen, PartialOrd, Ord)]
	pub struct SessionKeys {
		pub foo: UintAuthorityId,
	}
}

impl From<CandidateKeys> for SessionKeys {
	fn from(value: CandidateKeys) -> Self {
		let CandidateKey { bytes, .. } = &value.0[0];

		Self { foo: UintAuthorityId(bytes[0].into()) }
	}
}

impl From<SessionKeys> for CandidateKeys {
	fn from(value: SessionKeys) -> Self {
		Self(vec![CandidateKey { id: [0; 4], bytes: value.foo.0.to_le_bytes().to_vec() }])
	}
}

impl From<u64> for SessionKeys {
	fn from(value: u64) -> Self {
		SessionKeys { foo: UintAuthorityId::from(value) }
	}
}

#[allow(dead_code)]
#[frame_support::pallet]
pub mod mock_pallet {
	use frame_support::pallet_prelude::*;
	use sidechain_domain::ScEpochNumber;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	pub type CurrentEpoch<T: Config> = StorageValue<_, ScEpochNumber, ValueQuery>;
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		SessionCommitteeManagement: pallet,
		Session: pallet_session,
		Mock: mock_pallet,
	}
);

impl mock_pallet::Config for Test {}

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
	type AccountData = AccountData<u128>;
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

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = u128;
	type AccountStore = System;
}

impl pallet::Config for Test {
	type MaxValidators = ConstU32<32>;
	type AuthorityId = AuthorityId;
	type AuthorityKeys = SessionKeys;
	type MainChainScriptsOrigin = EnsureRoot<Self::AccountId>;

	fn select_authorities(
		input: AuthoritySelectionInputs,
		_sidechain_epoch: ScEpochNumber,
	) -> Option<BoundedVec<CommitteeMemberOf<Test>, Self::MaxValidators>> {
		// This is a good approximation of the real selection algorithm, that returns None iff there are no valid candidates to select from.
		if input.permissioned_candidates.is_empty() {
			None
		} else {
			let data: Vec<_> = (input.permissioned_candidates.iter().cloned())
				.map(|c| CommitteeMember::Permissioned {
					id: AuthorityId::from(ecdsa::Public::from(
						<[u8; 33]>::try_from(c.sidechain_public_key.0).unwrap(),
					)),
					keys: c.keys.into(),
				})
				.collect();
			Some(BoundedVec::truncate_from(data))
		}
	}

	fn current_epoch_number() -> ScEpochNumber {
		current_epoch_number()
	}

	type WeightInfo = ();

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const Period: u64 = 1;
	pub const Offset: u64 = 0;
}

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

impl pallet_session::Config for Test {
	type ValidatorId = AccountId;
	type ValidatorIdOf = sp_runtime::traits::ConvertInto;
	type ShouldEndSession = crate::Pallet<Test>;
	type NextSessionRotation = ();
	type SessionManager = crate::Pallet<Test>;
	type SessionHandler = TestSessionHandler;
	type Keys = SessionKeys;
	type DisablingStrategy = ();
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type KeyDeposit = ();
}

/// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	new_test_ext_with_genesis_initial_authorities(&[alice(), bob()])
}

/// Build genesis storage according to the mock runtime.
pub fn new_test_ext_with_genesis_initial_authorities(
	validators: &[MockValidator],
) -> sp_io::TestExternalities {
	let initial_authorities: Vec<CommitteeMemberOf<Test>> =
		validators.iter().map(MockValidator::permissioned).collect();
	let main_chain_scripts = MainChainScripts::default();
	let session_committee_management =
		SessionCommitteeManagementConfig { initial_authorities, main_chain_scripts };
	let t = RuntimeGenesisConfig { session_committee_management, ..Default::default() }
		.build_storage()
		.unwrap();
	t.into()
}

pub fn alice() -> MockValidator {
	MockValidator {
		name: "Alice",
		authority_keys: SessionKeys { foo: UintAuthorityId(0) },
		authority_id: CrossChainPublic::from(ecdsa::Public::from([0; 33])),
	}
}
pub fn bob() -> MockValidator {
	MockValidator {
		name: "Bob",
		authority_keys: SessionKeys { foo: UintAuthorityId(1) },
		authority_id: CrossChainPublic::from(ecdsa::Public::from([1; 33])),
	}
}
pub fn charlie() -> MockValidator {
	MockValidator {
		name: "Charlie",
		authority_keys: SessionKeys { foo: UintAuthorityId(2) },
		authority_id: CrossChainPublic::from(ecdsa::Public::from([2; 33])),
	}
}
pub fn dave() -> MockValidator {
	MockValidator {
		name: "Dave",
		authority_keys: SessionKeys { foo: UintAuthorityId(3) },
		authority_id: CrossChainPublic::from(ecdsa::Public::from([3; 33])),
	}
}
pub fn eve() -> MockValidator {
	MockValidator {
		name: "Eve",
		authority_keys: SessionKeys { foo: UintAuthorityId(4) },
		authority_id: CrossChainPublic::from(ecdsa::Public::from([4; 33])),
	}
}

#[derive(Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct MockValidator {
	pub name: &'static str,
	pub authority_keys: SessionKeys,
	pub authority_id: AuthorityId,
}

impl MockValidator {
	pub fn permissioned(&self) -> CommitteeMemberOf<Test> {
		CommitteeMember::Permissioned {
			id: self.authority_id.clone(),
			keys: self.authority_keys.clone(),
		}
	}

	pub fn account_id(&self) -> AccountId32 {
		self.authority_id.clone().into()
	}
}

pub fn as_permissioned_members(
	validators: &[MockValidator],
) -> BoundedVec<CommitteeMemberOf<Test>, ConstU32<32>> {
	BoundedVec::truncate_from(
		validators.iter().map(MockValidator::permissioned).collect::<Vec<_>>(),
	)
}
pub fn authority_ids(validators: &[MockValidator]) -> BoundedVec<AuthorityId, ConstU32<32>> {
	BoundedVec::truncate_from(validators.iter().map(|v| v.authority_id.clone()).collect())
}

pub fn set_validators_through_inherents(
	expected_authorities: &[MockValidator],
) -> PostDispatchInfo {
	let call = create_inherent_set_validators_call(expected_authorities).unwrap();

	call.dispatch_bypass_filter(RuntimeOrigin::none()).unwrap()
}

pub fn set_validators_directly(
	expected_validators: &[MockValidator],
	for_epoch: u64,
) -> DispatchResult {
	let expected_validators: Vec<_> =
		expected_validators.iter().map(MockValidator::permissioned).collect();
	let data_hash = SizedByteString(blake2_256(&expected_validators.encode()));
	SessionCommitteeManagement::set(
		RuntimeOrigin::none(),
		BoundedVec::truncate_from(expected_validators),
		for_epoch.into(),
		data_hash,
	)
}

pub fn create_inherent_data(validators: &[MockValidator]) -> (InherentData, SizedByteString<32>) {
	let mut inherent_data = InherentData::new();
	let data = AuthoritySelectionInputs {
		d_parameter: DParameter { num_permissioned_candidates: 10, num_registered_candidates: 10 },
		permissioned_candidates: validators
			.iter()
			.cloned()
			.map(|v| PermissionedCandidateData {
				sidechain_public_key: SidechainPublicKey(v.authority_id.into()),
				keys: v.authority_keys.into(),
			})
			.collect(),
		registered_candidates: vec![],
		epoch_nonce: EpochNonce::default(),
	};

	inherent_data
		.put_data(SessionCommitteeManagement::INHERENT_IDENTIFIER, &data)
		.unwrap();

	let data_hash = SizedByteString(blake2_256(&data.encode()));
	(inherent_data, data_hash)
}

pub fn create_inherent_set_validators_call(
	expected_authorities: &[MockValidator],
) -> Option<Call<Test>> {
	let inherent_data = create_inherent_data(expected_authorities);
	<SessionCommitteeManagement as ProvideInherent>::create_inherent(&inherent_data.0)
}

pub(crate) fn current_epoch_number() -> ScEpochNumber {
	mock_pallet::CurrentEpoch::<Test>::get()
}

#[track_caller]
pub(crate) fn start_session(session_index: u32) {
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

pub(crate) fn advance_one_block() {
	let block_number = System::block_number();
	System::on_finalize(block_number);
	Session::on_finalize(block_number);
	let parent_hash =
		if block_number > 1 { System::finalize().hash() } else { System::parent_hash() };
	System::reset_events();
	let next_block_number = block_number as u64 + 1;
	System::initialize(&next_block_number, &parent_hash, &Default::default());
	System::set_block_number(next_block_number.into());

	System::on_initialize(next_block_number);
	Session::on_initialize(next_block_number);
}
