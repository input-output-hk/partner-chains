use authority_selection_inherents::{AuthoritySelectionInputs, MaybeFromCandidateKeys};
use frame_support::{
	sp_runtime::traits::{BlakeTwo256, IdentityLookup},
	*,
};
use frame_system::EnsureRoot;
use pallet_balances::AccountData;
use pallet_session::{SessionHandler, ShouldEndSession};
use pallet_session_validator_management::CommitteeMemberOf;
use parity_scale_codec::MaxEncodedLen;

use sidechain_domain::ScEpochNumber;
use sp_core::{ConstU16, ConstU32, ConstU64, H256, ecdsa};
use sp_runtime::{
	AccountId32, BoundToRuntimeAppPublic, KeyTypeId, impl_opaque_keys, traits::ConvertInto,
};

pub struct CrossChainPublicLikeModule;
impl BoundToRuntimeAppPublic for CrossChainPublicLikeModule {
	type Public = sp_runtime::app_crypto::ecdsa::AppPublic;
}

impl_opaque_keys! {
	#[derive(Ord, PartialOrd, MaxEncodedLen)]
	pub struct CrossChainPublic {
		pub key: CrossChainPublicLikeModule,
	}
}

impl From<CrossChainPublic> for AccountId32 {
	fn from(value: CrossChainPublic) -> Self {
		AccountId32::new(value.blake2_256())
	}
}

impl From<ecdsa::Public> for CrossChainPublic {
	fn from(value: ecdsa::Public) -> Self {
		CrossChainPublic { key: value.into() }
	}
}

pub struct AuraLikeModule;
impl BoundToRuntimeAppPublic for AuraLikeModule {
	type Public = sp_runtime::app_crypto::sr25519::AppPublic;
}

pub struct GrandpaLikeModule;
impl BoundToRuntimeAppPublic for GrandpaLikeModule {
	type Public = sp_runtime::app_crypto::ed25519::AppPublic;
}

impl_opaque_keys! {
	#[derive(Ord, PartialOrd, MaxEncodedLen)]
	pub struct TestSessionKeys {
		pub aura: AuraLikeModule,
		pub grandpa: GrandpaLikeModule,
	}
}

impl MaybeFromCandidateKeys for TestSessionKeys {}

construct_runtime! {
	pub enum MockRuntime {
		System: frame_system,
		Balances: pallet_balances::pallet,
		Bridge: pallet_partner_chains_bridge::pallet,
		GovernedMap: pallet_governed_map::pallet,
		SessionCommitteeManagement: pallet_session_validator_management::pallet,
		Session: pallet_session::pallet,
	}
}

pub type Block = frame_system::mocking::MockBlock<MockRuntime>;

pub struct Mock;

impl frame_system::Config for MockRuntime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
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

impl pallet_sidechain::Config for MockRuntime {
	fn reference_timestamp_millis() -> u64 {
		0
	}
	type OnNewEpoch = ();
}

impl pallet_partner_chains_bridge::Config for MockRuntime {
	type GovernanceOrigin = EnsureRoot<MockRuntime>;
	type Recipient = AccountId32;
	type TransferHandler = ();
	type MaxTransfersPerBlock = ConstU32<3>;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

pub(crate) const TEST_MAX_CHANGES: u32 = 8;
pub(crate) type MaxChanges = ConstU32<TEST_MAX_CHANGES>;
pub(crate) type MaxKeyLength = ConstU32<64>;
pub(crate) type MaxValueLength = ConstU32<512>;

impl sp_governed_map::OnGovernedMappingChange<MaxKeyLength, MaxValueLength> for Mock {
	fn on_governed_mapping_change(
		_key: sidechain_domain::byte_string::BoundedString<MaxKeyLength>,
		_new_value: Option<BoundedVec<u8, MaxValueLength>>,
		_old_value: Option<BoundedVec<u8, MaxValueLength>>,
	) {
		unimplemented!()
	}
}

impl pallet_governed_map::Config for MockRuntime {
	type MaxChanges = MaxChanges;
	type MaxKeyLength = MaxKeyLength;
	type MaxValueLength = MaxValueLength;
	type OnGovernedMappingChange = ();
	type MainChainScriptsOrigin = EnsureRoot<Self::AccountId>;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

pub(crate) type MaxValidators = ConstU32<137>;

impl pallet_session_validator_management::Config for MockRuntime {
	type MaxValidators = MaxValidators;
	type AuthorityId = CrossChainPublic;
	type AuthorityKeys = TestSessionKeys;
	type MainChainScriptsOrigin = EnsureRoot<Self::AccountId>;
	type WeightInfo = ();

	fn select_authorities(
		_input: AuthoritySelectionInputs,
		_sidechain_epoch: ScEpochNumber,
	) -> Option<BoundedVec<CommitteeMemberOf<Self>, Self::MaxValidators>> {
		unimplemented!()
	}

	fn current_epoch_number() -> ScEpochNumber {
		unimplemented!()
	}

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for MockRuntime {
	type Balance = u128;
	type AccountStore = System;
}

impl pallet_session::Config for MockRuntime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId32;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = Mock;
	type NextSessionRotation = ();
	type SessionManager = ();
	type SessionHandler = Mock;
	type Keys = TestSessionKeys;
	type DisablingStrategy = pallet_session::disabling::UpToLimitWithReEnablingDisablingStrategy;
	type WeightInfo = pallet_session::weights::SubstrateWeight<MockRuntime>;
	type Currency = Balances;
	type KeyDeposit = ();
}

impl ShouldEndSession<u64> for Mock {
	fn should_end_session(_: u64) -> bool {
		false
	}
}

impl SessionHandler<AccountId32> for Mock {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] =
		&[KeyTypeId(*b"sr25"), KeyTypeId(*b"ed25")];

	fn on_genesis_session<Ks: sp_runtime::traits::OpaqueKeys>(_: &[(AccountId32, Ks)]) {}

	fn on_new_session<Ks: sp_runtime::traits::OpaqueKeys>(
		_changed: bool,
		_validators: &[(AccountId32, Ks)],
		_queued_validators: &[(AccountId32, Ks)],
	) {
	}

	fn on_disabled(_: u32) {}
}
