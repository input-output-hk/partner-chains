#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

extern crate alloc;

use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionInputs;
use authority_selection_inherents::filter_invalid_candidates::{
	validate_permissioned_candidate_data, PermissionedCandidateDataError, RegistrationDataError,
	StakeError,
};
use authority_selection_inherents::select_authorities::select_authorities;
use authority_selection_inherents::CommitteeMember;
use frame_support::genesis_builder_helper::{build_state, get_preset};
use frame_support::inherent::ProvideInherent;
use frame_support::traits::ValidatorSet;
use frame_support::weights::constants::RocksDbWeight as RuntimeDbWeight;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, ValidatorSetWithIdentification},
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, IdentityFee},
	BoundedVec,
};
use opaque::SessionKeys;
use pallet_grandpa::AuthorityId as GrandpaId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session_validator_management;
use pallet_session_validator_management::session_manager::ValidatorManagementSessionManager;
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
use parity_scale_codec::MaxEncodedLen;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::Deserialize;
use sidechain_domain::byte_string::{BoundedString, SizedByteString};
use sidechain_domain::{
	DelegatorKey, MainchainKeyHash, PermissionedCandidateData, RegistrationData, ScEpochNumber,
	ScSlotNumber, StakeDelegation, StakePoolPublicKey, UtxoId,
};
use sidechain_slots::Slot;
use sp_api::impl_runtime_apis;
use sp_block_participation::AsCardanoSPO;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
#[cfg(feature = "runtime-benchmarks")]
use sp_core::ByteArray;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_inherents::InherentIdentifier;
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{
		AccountIdLookup, BlakeTwo256, Block as BlockT, Convert, IdentifyAccount, NumberFor, One,
		OpaqueKeys, Verify,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiSignature, Perbill,
};
use sp_sidechain::SidechainStatus;
use sp_staking::SessionIndex;
use sp_std::prelude::*;
use sp_version::RuntimeVersion;
use sp_weights::Weight;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod genesis_config_presets;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod header_tests;

mod test_helper_pallet;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;
	use parity_scale_codec::MaxEncodedLen;
	use sp_core::{ed25519, sr25519};
	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	pub const CROSS_CHAIN: KeyTypeId = KeyTypeId(*b"crch");
	pub struct CrossChainRuntimeAppPublic;

	pub mod cross_chain_app {
		use super::CROSS_CHAIN;
		use parity_scale_codec::MaxEncodedLen;
		use sidechain_domain::SidechainPublicKey;
		use sp_core::crypto::AccountId32;
		use sp_runtime::app_crypto::{app_crypto, ecdsa};
		use sp_runtime::traits::IdentifyAccount;
		use sp_runtime::MultiSigner;
		use sp_std::vec::Vec;

		app_crypto!(ecdsa, CROSS_CHAIN);
		impl MaxEncodedLen for Signature {
			fn max_encoded_len() -> usize {
				ecdsa::Signature::max_encoded_len()
			}
		}

		impl From<Signature> for Vec<u8> {
			fn from(value: Signature) -> Self {
				value.into_inner().0.to_vec()
			}
		}

		impl From<Public> for AccountId32 {
			fn from(value: Public) -> Self {
				MultiSigner::from(ecdsa::Public::from(value)).into_account()
			}
		}

		impl From<Public> for Vec<u8> {
			fn from(value: Public) -> Self {
				value.into_inner().0.to_vec()
			}
		}

		impl TryFrom<SidechainPublicKey> for Public {
			type Error = SidechainPublicKey;
			fn try_from(pubkey: SidechainPublicKey) -> Result<Self, Self::Error> {
				let cross_chain_public_key =
					Public::try_from(pubkey.0.as_slice()).map_err(|_| pubkey)?;
				Ok(cross_chain_public_key)
			}
		}
	}

	impl_opaque_keys! {
		#[derive(MaxEncodedLen, PartialOrd, Ord)]
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
			pub im_online: ImOnline,
		}
	}
	impl From<(sr25519::Public, ed25519::Public, sr25519::Public)> for SessionKeys {
		fn from(
			(aura, grandpa, im_online): (sr25519::Public, ed25519::Public, sr25519::Public),
		) -> Self {
			Self { aura: aura.into(), grandpa: grandpa.into(), im_online: im_online.into() }
		}
	}

	impl_opaque_keys! {
		pub struct CrossChainKey {
			pub account: CrossChainPublic,
		}
	}
}

pub type CrossChainPublic = opaque::cross_chain_app::Public;

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("cardano-sidechain"),
	impl_name: alloc::borrow::Cow::Borrowed("cardano-sidechain"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 160,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

/// We allow for 2 seconds of compute with a 6 second average block time.
pub const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);
pub const MAXIMUM_BLOCK_LENGTH: u32 = 5 * 1024 * 1024;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights
		::with_sensible_defaults(MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(MAXIMUM_BLOCK_LENGTH, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// The block type for the runtime.
	type Block = Block;
	/// The type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RuntimeDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
	// WeightInfo for extensions is present but not accessible in polkadot-stable2412-1, because of that we are using () in our demo runtime
	type ExtensionsWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type RuntimeTask = RuntimeTask;
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

impl pallet_native_token_management::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type TokenTransferHandler = TestHelperPallet;
	type WeightInfo = pallet_native_token_management::weights::SubstrateWeight<Runtime>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxValidators;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type SlotDuration = ConstU64<SLOT_DURATION>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
	type MaxAuthorities = MaxValidators;
	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = ConstU64<0>;

	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: u128 = 500;

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
	type WeightInfo = pallet_transaction_payment::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

impl pallet_partner_chains_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ShouldEndSession = ValidatorManagementSessionManager<Runtime>;
	type NextSessionRotation = pallet_session::PeriodicSessions<PERIOD, OFFSET>;
	type SessionManager = ValidatorManagementSessionManager<Runtime>;
	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
}

parameter_types! {
	pub const MaxValidators: u32 = 32;
}

impl pallet_session_validator_management::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxValidators = MaxValidators;
	type AuthorityId = CrossChainPublic;
	type AuthorityKeys = SessionKeys;
	type AuthoritySelectionInputs = AuthoritySelectionInputs;
	type ScEpochNumber = ScEpochNumber;
	type WeightInfo = pallet_session_validator_management::weights::SubstrateWeight<Runtime>;
	type CommitteeMember = CommitteeMember<CrossChainPublic, SessionKeys>;

	fn select_authorities(
		input: AuthoritySelectionInputs,
		sidechain_epoch: ScEpochNumber,
	) -> Option<BoundedVec<Self::CommitteeMember, Self::MaxValidators>> {
		Some(BoundedVec::truncate_from(
			select_authorities(Sidechain::genesis_utxo(), input, sidechain_epoch)?
				.into_iter()
				.map(|member| member.into())
				.collect(),
		))
	}

	fn current_epoch_number() -> ScEpochNumber {
		Sidechain::current_epoch_number()
	}
}

parameter_types! {
	pub const TokenConversionRate: u128 = 1_000_000_000u128;
	pub const MaxTransactions: u32 = 256u32;
}

impl pallet_sidechain::Config for Runtime {
	fn current_slot_number() -> ScSlotNumber {
		ScSlotNumber(*pallet_aura::CurrentSlot::<Self>::get())
	}
	type OnNewEpoch = TestHelperPallet;
}

pub type BeneficiaryId = sidechain_domain::byte_string::SizedByteString<32>;

#[derive(
	MaxEncodedLen, Encode, Decode, Clone, TypeInfo, PartialEq, Eq, Debug, Hash, PartialOrd, Ord,
)]
pub enum BlockAuthor {
	Incentivized(CrossChainPublic, StakePoolPublicKey),
	ProBono(CrossChainPublic),
}
impl BlockAuthor {
	pub fn id(&self) -> &CrossChainPublic {
		match self {
			Self::Incentivized(id, _) => id,
			Self::ProBono(id) => id,
		}
	}
}
impl From<CommitteeMember<CrossChainPublic, SessionKeys>> for BlockAuthor {
	fn from(value: CommitteeMember<CrossChainPublic, SessionKeys>) -> Self {
		match value {
			CommitteeMember::Permissioned { id, .. } => BlockAuthor::ProBono(id),
			CommitteeMember::Registered { id, stake_pool_pub_key, .. } => {
				BlockAuthor::Incentivized(id, stake_pool_pub_key)
			},
		}
	}
}

impl AsCardanoSPO for BlockAuthor {
	fn as_cardano_spo(&self) -> Option<MainchainKeyHash> {
		match self {
			BlockAuthor::Incentivized(_, key) => Some(key.hash()),
			BlockAuthor::ProBono(_) => None,
		}
	}
}

pub const MAX_METADATA_URL_LENGTH: u32 = 512;

#[derive(Clone, Debug, MaxEncodedLen, Encode, Deserialize)]
pub struct BlockProducerMetadata {
	pub url: BoundedString<MAX_METADATA_URL_LENGTH>,
	pub hash: SizedByteString<32>,
}

#[cfg(feature = "runtime-benchmarks")]
pub struct PalletBlockProductionLogBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_block_production_log::benchmarking::BenchmarkHelper<BlockAuthor>
	for PalletBlockProductionLogBenchmarkHelper
{
	fn producer_id() -> BlockAuthor {
		let id = sp_core::ecdsa::Public::from_slice(&[0u8; 33]).unwrap().into();
		BlockAuthor::ProBono(id)
	}
}

impl pallet_block_production_log::Config for Runtime {
	type BlockProducerId = BlockAuthor;
	type WeightInfo = pallet_block_production_log::weights::SubstrateWeight<Runtime>;

	fn current_slot() -> sp_consensus_slots::Slot {
		let slot: u64 = pallet_aura::CurrentSlot::<Runtime>::get().into();
		sp_consensus_slots::Slot::from(slot)
	}

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = PalletBlockProductionLogBenchmarkHelper;
}

impl pallet_address_associations::Config for Runtime {
	type WeightInfo = pallet_address_associations::weights::SubstrateWeight<Runtime>;

	type PartnerChainAddress = AccountId;

	fn genesis_utxo() -> UtxoId {
		Sidechain::genesis_utxo()
	}
}

impl pallet_block_participation::Config for Runtime {
	type WeightInfo = pallet_block_participation::weights::SubstrateWeight<Runtime>;
	type BlockAuthor = BlockAuthor;
	type DelegatorId = DelegatorKey;

	fn should_release_data(slot: sidechain_slots::Slot) -> Option<sidechain_slots::Slot> {
		TestHelperPallet::should_release_participation_data(slot)
	}

	fn blocks_produced_up_to_slot(slot: Slot) -> impl Iterator<Item = (Slot, BlockAuthor)> {
		BlockProductionLog::peek_prefix(slot)
	}

	fn discard_blocks_produced_up_to_slot(slot: Slot) {
		BlockProductionLog::drop_prefix(&slot)
	}

	const TARGET_INHERENT_ID: InherentIdentifier = TestHelperPallet::INHERENT_IDENTIFIER;
}

impl crate::test_helper_pallet::Config for Runtime {}

impl<LocalCall> frame_system::offchain::CreateInherent<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_inherent(call: RuntimeCall) -> UncheckedExtrinsic {
		generic::UncheckedExtrinsic::new_bare(call).into()
	}
}

impl<C> frame_system::offchain::CreateTransactionBase<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type RuntimeCall = RuntimeCall;
}

parameter_types! {
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	pub const MaxAuthorities: u32 = 1000;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const PERIOD: u32 = MINUTES / 2;
	pub const OFFSET: u32 = 0;
}

pallet_partner_chains_session::impl_pallet_session_config!(Runtime);

pub struct ValidatorIdOf;

impl<T> Convert<T, Option<T>> for ValidatorIdOf {
	fn convert(t: T) -> Option<T> {
		Some(t)
	}
}

pub struct GlueCode;

impl ValidatorSet<AccountId> for GlueCode {
	type ValidatorId = AccountId;
	type ValidatorIdOf = ValidatorIdOf;

	fn session_index() -> SessionIndex {
		pallet_session::Pallet::<Runtime>::current_index()
	}
	fn validators() -> Vec<Self::ValidatorId> {
		let (_epoch, validators) =
			pallet_session_validator_management::Pallet::<Runtime>::get_current_committee();

		use sp_session_validator_management::CommitteeMember;
		validators
			.into_iter()
			.map(|committee_member| committee_member.authority_id().into())
			.collect()
	}
}

impl ValidatorSetWithIdentification<AccountId> for GlueCode {
	type Identification = AccountId;
	type IdentificationOf = ValidatorIdOf;
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type RuntimeEvent = RuntimeEvent;
	type NextSessionRotation = pallet_session::PeriodicSessions<PERIOD, OFFSET>;
	type ValidatorSet = GlueCode;
	type ReportUnresponsiveness = ();
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_partner_chains_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = (ImOnline,);
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime {
		System: frame_system,
		Timestamp: pallet_timestamp,
		Authorship: pallet_authorship,
		Aura: pallet_aura,
		Grandpa: pallet_grandpa,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
		Sudo: pallet_sudo,
		// Custom Pallets
		// Sidechain pallet must come after the Aura pallet, since it gets the slot number from it
		Sidechain: pallet_sidechain,
		SessionCommitteeManagement: pallet_session_validator_management,
		AddressAssociations: pallet_address_associations,
		BlockProductionLog: pallet_block_production_log,
		BlockParticipation: pallet_block_participation,
		// pallet_grandpa reads pallet_session::pallet::CurrentIndex storage.
		// Only stub implementation of pallet_session should be wired.
		// Partner Chains session_manager ValidatorManagementSessionManager writes to pallet_session::pallet::CurrentIndex.
		// ValidatorManagementSessionManager is wired in by pallet_partner_chains_session.
		PalletSession: pallet_session,
		// The order matters!! pallet_partner_chains_session needs to come last for correct initialization order
		Session: pallet_partner_chains_session,
		ImOnline: pallet_im_online,
		NativeTokenManagement: pallet_native_token_management,
		TestHelperPallet: crate::test_helper_pallet,
	}
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
pub type Migrations = (
	pallet_session_validator_management::migrations::v1::LegacyToV1Migration<Runtime>,
	// More migrations can be added here
);
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_sudo, Sudo]
		[pallet_native_token_management, NativeTokenManagement]
		[pallet_block_production_log, BlockProductionLog]
		[pallet_address_associations, AddressAssociations]
		[pallet_block_participation, BlockParticipation]
	);
}

impl_runtime_apis! {
	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, crate::genesis_config_presets::get_preset)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			crate::genesis_config_presets::preset_names()
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			pallet_aura::Authorities::<Runtime>::get().into_inner()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			// despite being named "generate" this function also adds generated keys to local keystore
			opaque::CrossChainKey::generate(seed.clone());
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			_authority_id: GrandpaId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}


	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: sp_weights::Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: sp_weights::Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch};
			use sp_storage::TrackedStorageKey;

			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;
			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}

	impl sp_sidechain::GetGenesisUtxo<Block> for Runtime {
		fn genesis_utxo() -> UtxoId {
			Sidechain::genesis_utxo()
		}
	}

	impl sp_sidechain::GetSidechainStatus<Block> for Runtime {
		fn get_sidechain_status() -> SidechainStatus {
			SidechainStatus {
				epoch: Sidechain::current_epoch_number(),
				slot: ScSlotNumber(*pallet_aura::CurrentSlot::<Runtime>::get()),
				slots_per_epoch: Sidechain::slots_per_epoch().0,
			}
		}
	}

	impl sidechain_slots::SlotApi<Block> for Runtime {
		fn slot_config() -> sidechain_slots::ScSlotConfig {
			sidechain_slots::ScSlotConfig {
				slots_per_epoch: Sidechain::slots_per_epoch(),
				slot_duration: <Self as sp_consensus_aura::runtime_decl_for_aura_api::AuraApi<Block, AuraId>>::slot_duration()
			}
		}
	}

	#[api_version(2)]
	impl sp_session_validator_management::SessionValidatorManagementApi<
		Block,
		CommitteeMember<CrossChainPublic, SessionKeys>,
		AuthoritySelectionInputs,
		sidechain_domain::ScEpochNumber
	> for Runtime {
		fn get_current_committee() -> (ScEpochNumber, Vec<CommitteeMember<CrossChainPublic, SessionKeys>>) {
			SessionCommitteeManagement::get_current_committee()
		}
		fn get_next_committee() -> Option<(ScEpochNumber, Vec<CommitteeMember<CrossChainPublic, SessionKeys>>)> {
			SessionCommitteeManagement::get_next_committee()
		}
		fn get_next_unset_epoch_number() -> sidechain_domain::ScEpochNumber {
			SessionCommitteeManagement::get_next_unset_epoch_number()
		}
		fn calculate_committee(authority_selection_inputs: AuthoritySelectionInputs, sidechain_epoch: ScEpochNumber) -> Option<Vec<CommitteeMember<CrossChainPublic, SessionKeys>>> {
			SessionCommitteeManagement::calculate_committee(authority_selection_inputs, sidechain_epoch)
		}
		fn get_main_chain_scripts() -> sp_session_validator_management::MainChainScripts {
			SessionCommitteeManagement::get_main_chain_scripts()
		}
	}

	impl authority_selection_inherents::filter_invalid_candidates::CandidateValidationApi<Block> for Runtime {
		fn validate_registered_candidate_data(stake_pool_public_key: &StakePoolPublicKey, registration_data: &RegistrationData) -> Option<RegistrationDataError> {
			authority_selection_inherents::filter_invalid_candidates::validate_registration_data(stake_pool_public_key, registration_data, Sidechain::genesis_utxo()).err()
		}
		fn validate_stake(stake: Option<StakeDelegation>) -> Option<StakeError> {
			authority_selection_inherents::filter_invalid_candidates::validate_stake(stake).err()
		}
		fn validate_permissioned_candidate_data(candidate: PermissionedCandidateData) -> Option<PermissionedCandidateDataError> {
			validate_permissioned_candidate_data::<CrossChainPublic>(candidate).err()
		}
	}

	impl sp_native_token_management::NativeTokenManagementApi<Block> for Runtime {
		fn get_main_chain_scripts() -> Option<sp_native_token_management::MainChainScripts> {
			NativeTokenManagement::get_main_chain_scripts()
		}
		fn initialized() -> bool {
			NativeTokenManagement::initialized()
		}
	}

	impl sp_block_production_log::BlockProductionLogApi<Block, CommitteeMember<CrossChainPublic, SessionKeys>>  for Runtime {
		fn get_author(slot: Slot) -> Option<CommitteeMember<CrossChainPublic, SessionKeys>> {
			 SessionCommitteeManagement::get_current_authority_round_robin(*slot as usize)
		}
	}

	impl sp_block_participation::BlockParticipationApi<Block, BlockAuthor> for Runtime {
		fn should_release_data(slot: Slot) -> Option<Slot> {
			BlockParticipation::should_release_data(slot)
		}
		fn blocks_produced_up_to_slot(slot: Slot) -> Vec<(Slot, BlockAuthor)> {
			<Runtime as pallet_block_participation::Config>::blocks_produced_up_to_slot(slot).collect()
		}
		fn target_inherent_id() -> InherentIdentifier {
			<Runtime as pallet_block_participation::Config>::TARGET_INHERENT_ID
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::mock::*;
	use frame_support::{
		dispatch::PostDispatchInfo,
		inherent::ProvideInherent,
		traits::{UnfilteredDispatchable, WhitelistedStorageKeys},
	};
	use sp_core::{hexdisplay::HexDisplay, Pair};
	use sp_inherents::InherentData;
	use std::collections::HashSet;

	#[test]
	fn check_whitelist() {
		let whitelist: HashSet<String> = super::AllPalletsWithSystem::whitelisted_storage_keys()
			.iter()
			.map(|e| HexDisplay::from(&e.key).to_string())
			.collect();

		// Block Number
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac")
		);
		// Total Issuance
		assert!(
			whitelist.contains("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80")
		);
		// Execution Phase
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a")
		);
		// Event Count
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850")
		);
		// System Events
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7")
		);
	}

	// The set committee takes effect next session. Committee can be set for 1 session in advance.
	#[test]
	fn check_grandpa_authorities_rotation() {
		new_test_ext().execute_with(|| {
			// Needs to be run to initialize first slot and epoch numbers;
			advance_block();
			set_committee_through_inherent_data(&[alice()]);
			until_epoch_after_finalizing(1, &|| {
				assert_current_epoch!(0);
				assert_grandpa_weights();
				assert_grandpa_authorities!([alice(), bob()]);
			});

			set_committee_through_inherent_data(&[bob()]);
			for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH, &|| {
				assert_current_epoch!(1);
				assert_grandpa_weights();
				assert_grandpa_authorities!([alice()]);
			});

			for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH, &|| {
				assert_current_epoch!(2);
				assert_grandpa_weights();
				assert_grandpa_authorities!([bob()]);
			});

			// Authorities can be set as late as in the first block of new epoch, but it makes session last 1 block longer
			set_committee_through_inherent_data(&[alice()]);
			advance_block();
			assert_current_epoch!(3);
			assert_grandpa_authorities!([bob()]);
			set_committee_through_inherent_data(&[alice(), bob()]);
			for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH - 1, &|| {
				assert_current_epoch!(3);
				assert_grandpa_weights();
				assert_grandpa_authorities!([alice()]);
			});

			for_next_n_blocks_after_finalizing(SLOTS_PER_EPOCH * 3, &|| {
				assert_grandpa_weights();
				assert_grandpa_authorities!([alice(), bob()]);
			});
		});

		fn assert_grandpa_weights() {
			Grandpa::grandpa_authorities()
				.into_iter()
				.for_each(|(_, weight)| assert_eq!(weight, 1))
		}
	}

	// The set committee takes effect next session. Committee can be set for 1 session in advance.
	#[test]
	fn check_aura_authorities_rotation() {
		new_test_ext().execute_with(|| {
			advance_block();
			set_committee_through_inherent_data(&[alice()]);
			until_epoch(1, &|| {
				assert_current_epoch!(0);
				assert_aura_authorities!([alice(), bob()]);
			});

			for_next_n_blocks(SLOTS_PER_EPOCH, &|| {
				assert_current_epoch!(1);
				assert_aura_authorities!([alice()]);
			});

			// Authorities can be set as late as in the first block of new epoch, but it makes session last 1 block longer
			set_committee_through_inherent_data(&[bob()]);
			assert_current_epoch!(2);
			assert_aura_authorities!([alice()]);
			advance_block();
			set_committee_through_inherent_data(&[alice(), bob()]);
			for_next_n_blocks(SLOTS_PER_EPOCH - 1, &|| {
				assert_current_epoch!(2);
				assert_aura_authorities!([bob()]);
			});

			set_committee_through_inherent_data(&[alice(), bob()]);
			for_next_n_blocks(SLOTS_PER_EPOCH * 3, &|| {
				assert_aura_authorities!([alice(), bob()]);
			});
		});
	}

	// The set committee takes effect at next session. Committee can be set for 1 session in advance.
	#[test]
	fn check_cross_chain_committee_rotation() {
		new_test_ext().execute_with(|| {
			advance_block();
			set_committee_through_inherent_data(&[alice()]);
			until_epoch(1, &|| {
				assert_current_epoch!(0);
				assert_next_committee!([alice()]);
			});

			set_committee_through_inherent_data(&[bob()]);
			for_next_n_blocks(SLOTS_PER_EPOCH, &|| {
				assert_current_epoch!(1);
				assert_next_committee!([bob()]);
			});

			set_committee_through_inherent_data(&[]);
			for_next_n_blocks(SLOTS_PER_EPOCH, &|| {
				assert_current_epoch!(2);
				assert_next_committee!([bob()]);
			});
		});
	}

	pub fn set_committee_through_inherent_data(
		expected_authorities: &[TestKeys],
	) -> PostDispatchInfo {
		let epoch = Sidechain::current_epoch_number();
		let slot = *pallet_aura::CurrentSlot::<Test>::get();
		println!(
			"(slot {slot}, epoch {epoch}) Setting {} authorities for next epoch",
			expected_authorities.len()
		);
		let inherent_data_struct = create_inherent_data_struct(expected_authorities);
		let mut inherent_data = InherentData::new();
		inherent_data
			.put_data(
				SessionCommitteeManagement::INHERENT_IDENTIFIER,
				&inherent_data_struct.data.unwrap(),
			)
			.expect("Setting inherent data should not fail");
		let call = <SessionCommitteeManagement as ProvideInherent>::create_inherent(&inherent_data)
			.expect("Creating test inherent should not fail");
		println!("    inherent: {:?}", call);
		call.dispatch_bypass_filter(RuntimeOrigin::none())
			.expect("dispatching test call should work")
	}
}
