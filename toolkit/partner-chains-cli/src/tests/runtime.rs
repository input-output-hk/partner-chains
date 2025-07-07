#![cfg(not(feature = "runtime-benchmarks"))]
use frame_support::{
	sp_runtime::traits::{BlakeTwo256, IdentityLookup},
	*,
};
use frame_system::EnsureRoot;
use pallet_partner_chains_session::{SessionHandler, ShouldEndSession};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::Serialize;
use sidechain_domain::{ScEpochNumber, ScSlotNumber};
use sp_core::{ConstU16, ConstU32, ConstU64, H256, ed25519, sr25519};
use sp_runtime::{AccountId32, traits::OpaqueKeys};

#[derive(
	Clone,
	Debug,
	Decode,
	DecodeWithMemTracking,
	Default,
	Deserialize,
	Encode,
	Eq,
	MaxEncodedLen,
	Ord,
	PartialEq,
	PartialOrd,
	Serialize,
	TypeInfo,
)]
pub struct TestSessionKeys {
	pub aura: sr25519::Public,
	pub grandpa: ed25519::Public,
}

impl OpaqueKeys for TestSessionKeys {
	type KeyTypeIdProviders = ();

	fn key_ids() -> &'static [sp_runtime::KeyTypeId] {
		&[]
	}

	fn get_raw(&self, _: sp_runtime::KeyTypeId) -> &[u8] {
		&[]
	}
}

construct_runtime! {
	pub enum MockRuntime {
		System: frame_system,
		NativeToken: pallet_native_token_management::pallet,
		GovernedMap: pallet_governed_map::pallet,
		SessionCommitteeManagement: pallet_session_validator_management::pallet,
		Session: pallet_partner_chains_session::pallet,
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

impl pallet_sidechain::Config for MockRuntime {
	fn current_slot_number() -> ScSlotNumber {
		ScSlotNumber(0)
	}
	type OnNewEpoch = ();
}

impl pallet_native_token_management::TokenTransferHandler for Mock {
	fn handle_token_transfer(_: sidechain_domain::NativeTokenAmount) -> dispatch::DispatchResult {
		unimplemented!()
	}
}

impl pallet_native_token_management::Config for MockRuntime {
	type RuntimeEvent = RuntimeEvent;
	type MainChainScriptsOrigin = EnsureRoot<Self::AccountId>;
	type TokenTransferHandler = Mock;
	type WeightInfo = ();
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
}

pub(crate) type MaxValidators = ConstU32<137>;

impl pallet_session_validator_management::Config for MockRuntime {
	type RuntimeEvent = RuntimeEvent;
	type MaxValidators = MaxValidators;
	type AuthorityId = AccountId32;
	type AuthorityKeys = (sr25519::Public, ed25519::Public);
	type AuthoritySelectionInputs = ();
	type ScEpochNumber = ScEpochNumber;
	type CommitteeMember = (AccountId32, (sr25519::Public, ed25519::Public));
	type MainChainScriptsOrigin = EnsureRoot<Self::AccountId>;
	type WeightInfo = ();

	fn select_authorities(
		_input: Self::AuthoritySelectionInputs,
		_sidechain_epoch: Self::ScEpochNumber,
	) -> Option<BoundedVec<Self::CommitteeMember, Self::MaxValidators>> {
		unimplemented!()
	}

	fn current_epoch_number() -> Self::ScEpochNumber {
		unimplemented!()
	}
}

impl pallet_partner_chains_session::Config for MockRuntime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId32;
	type ShouldEndSession = Mock;
	type NextSessionRotation = ();
	type SessionManager = ();
	type SessionHandler = Mock;
	type Keys = TestSessionKeys;
}

impl ShouldEndSession<u64> for Mock {
	fn should_end_session(_: u64) -> bool {
		false
	}
}

impl SessionHandler<AccountId32> for Mock {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[];

	fn on_genesis_session<Ks: sp_runtime::traits::OpaqueKeys>(_: &[(AccountId32, Ks)]) {}

	fn on_new_session<Ks: sp_runtime::traits::OpaqueKeys>(
		_changed: bool,
		_validators: &[(AccountId32, Ks)],
		_queued_validators: &[(AccountId32, Ks)],
	) {
	}

	fn on_disabled(_: u32) {}
}
