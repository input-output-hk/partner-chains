use crate::{pallet, Call};
use frame_support::{
	dispatch::PostDispatchInfo,
	pallet_prelude::*,
	parameter_types,
	traits::{ConstU64, UnfilteredDispatchable},
};
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use sp_session_validator_management::MainChainScripts;

type Block = frame_system::mocking::MockBlock<Test>;

type AccountId = u64;
type AuthorityId = u64;
pub type ScEpochNumber = u64;
pub type AuthorityKeys = u64;

#[allow(dead_code)]
#[frame_support::pallet]
pub mod mock_pallet {
	use frame_support::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	#[pallet::getter(fn current_epoch)]
	pub type CurrentEpoch<T: Config> = StorageValue<_, u64, ValueQuery>;
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		SessionCommitteeManagement: pallet,
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
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
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

impl pallet::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MaxValidators = ConstU32<32>;
	type AuthorityId = AuthorityId;
	type AuthorityKeys = AuthorityKeys;
	type AuthoritySelectionInputs =
		BoundedVec<(Self::AuthorityId, Self::AuthorityKeys), Self::MaxValidators>;
	type ScEpochNumber = ScEpochNumber;

	fn select_authorities(
		input: Self::AuthoritySelectionInputs,
		_sidechain_epoch: Self::ScEpochNumber,
	) -> Option<BoundedVec<(Self::AuthorityId, Self::AuthorityKeys), Self::MaxValidators>> {
		// This is a good approximation of the real selection algorithm, that returs None iff there are no valid candidates to select from.
		if input.is_empty() {
			None
		} else {
			Some(input)
		}
	}

	fn current_epoch_number() -> Self::ScEpochNumber {
		current_epoch_number()
	}

	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	new_test_ext_with_genesis_initial_authorities(&[ALICE, BOB])
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext_with_genesis_initial_authorities(
	validators: &[MockValidator],
) -> sp_io::TestExternalities {
	let initial_authorities: Vec<(AuthorityId, AuthorityKeys)> =
		validators.iter().map(MockValidator::ids_and_keys).collect();
	let main_chain_scripts = MainChainScripts::default();
	let session_committee_management =
		SessionCommitteeManagementConfig { initial_authorities, main_chain_scripts };
	let t = RuntimeGenesisConfig { session_committee_management, ..Default::default() }
		.build_storage()
		.unwrap();
	t.into()
}

pub const ALICE: MockValidator =
	MockValidator { name: "Alice", authority_keys: 11, authority_id: 21 };
pub const BOB: MockValidator = MockValidator { name: "Bob", authority_keys: 12, authority_id: 22 };
pub const CHARLIE: MockValidator =
	MockValidator { name: "Charlie", authority_keys: 13, authority_id: 23 };
pub const DAVE: MockValidator =
	MockValidator { name: "Dave", authority_keys: 14, authority_id: 24 };
pub const EVE: MockValidator = MockValidator { name: "Eve", authority_keys: 15, authority_id: 25 };

#[derive(Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct MockValidator {
	pub name: &'static str,
	pub authority_keys: AuthorityKeys,
	pub authority_id: AuthorityId,
}

impl MockValidator {
	pub fn ids_and_keys(&self) -> (AuthorityId, AuthorityKeys) {
		(self.authority_id, self.authority_keys)
	}
}

pub fn ids_and_keys_fn(
	validators: &[MockValidator],
) -> BoundedVec<(AuthorityId, AuthorityKeys), ConstU32<32>> {
	BoundedVec::truncate_from(
		validators.iter().map(MockValidator::ids_and_keys).collect::<Vec<_>>(),
	)
}
pub fn authority_ids(validators: &[MockValidator]) -> BoundedVec<AuthorityId, ConstU32<32>> {
	BoundedVec::truncate_from(validators.iter().map(|v| v.authority_id).collect())
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
		expected_validators.iter().map(MockValidator::ids_and_keys).collect();
	SessionCommitteeManagement::set(
		RuntimeOrigin::none(),
		BoundedVec::truncate_from(expected_validators),
		for_epoch,
	)
}

pub fn create_inherent_data(validators: &[MockValidator]) -> InherentData {
	let mut inherent_data = InherentData::new();
	let data: BoundedVec<_, ConstU32<32>> =
		BoundedVec::truncate_from(validators.iter().map(MockValidator::ids_and_keys).collect());
	inherent_data
		.put_data(SessionCommitteeManagement::INHERENT_IDENTIFIER, &data)
		.unwrap();
	inherent_data
}

pub fn create_inherent_set_validators_call(
	expected_authorities: &[MockValidator],
) -> Option<Call<Test>> {
	let inherent_data = create_inherent_data(expected_authorities);
	<SessionCommitteeManagement as ProvideInherent>::create_inherent(&inherent_data)
}

pub(crate) fn current_epoch_number() -> u64 {
	Mock::current_epoch()
}
