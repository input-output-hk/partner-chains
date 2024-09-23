#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

// A few exports that help ease life for downstream crates.
use crate::weights::rocksdb_weights::constants::RocksDbWeight;
use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionInputs;
use authority_selection_inherents::filter_invalid_candidates::{
	validate_permissioned_candidate_data, PermissionedCandidateDataError, RegistrationDataError,
	StakeError,
};
use authority_selection_inherents::select_authorities::select_authorities;
use chain_params::SidechainParams;
use frame_support::genesis_builder_helper::{build_state, get_preset};
use frame_support::traits::fungible::Balanced;
use frame_support::traits::tokens::Precision;
use frame_support::BoundedVec;
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, KeyOwnerProofSystem, Randomness,
		StorageInfo,
	},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_REF_TIME_PER_SECOND},
		IdentityFee,
	},
	PalletId, StorageValue,
};
pub use frame_system::Call as SystemCall;
use hex_literal::hex;
use opaque::SessionKeys;
pub use pallet_balances::Call as BalancesCall;
use pallet_grandpa::AuthorityId as GrandpaId;
pub use pallet_session_validator_management;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
use session_manager::ValidatorManagementSessionManager;
use sidechain_domain::{
	MainchainPublicKey, NativeTokenAmount, PermissionedCandidateData, RegistrationData,
	ScEpochNumber, ScSlotNumber, StakeDelegation,
};
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::DispatchResult;
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, NumberFor, One, OpaqueKeys,
		Verify,
	},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiSignature,
};
pub use sp_runtime::{Perbill, Permill};
use sp_sidechain::SidechainStatus;
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use sp_weights::Weight;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

#[allow(warnings)]
mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod header_tests;

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

type DbWeight = RocksDbWeight;

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
		}
	}
	impl From<(sr25519::Public, ed25519::Public)> for SessionKeys {
		fn from((aura, grandpa): (sr25519::Public, ed25519::Public)) -> Self {
			Self { aura: aura.into(), grandpa: grandpa.into() }
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
	spec_name: create_runtime_str!("cardano-sidechain"),
	impl_name: create_runtime_str!("cardano-sidechain"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 113,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
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

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

/// We allow for 2 seconds of compute with a 6 second average block time.
pub const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights::builder()
			.base_block(weights::block_weights::BlockExecutionWeight::get())
			.for_class(frame_support::dispatch::DispatchClass::all(), |weights| {
				weights.base_extrinsic = weights::extrinsic_weights::ExtrinsicBaseWeight::get();
			})
			.for_class(frame_support::dispatch::DispatchClass::Normal, |weights| {
				weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
			})
			.for_class(frame_support::dispatch::DispatchClass::Operational, |weights| {
				weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
				weights.reserved = Some(
					MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT,
				);
			})
			.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
			.build_or_panic();
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
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
	type DbWeight = DbWeight;
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
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
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

pub struct TokenTransferHandler;

impl pallet_native_token_management::TokenTransferHandler for TokenTransferHandler {
	fn handle_token_transfer(token_amount: NativeTokenAmount) -> DispatchResult {
		// Mint the "transfered" tokens into a dummy address.
		// This is done for visibility in tests only.
		// Despite using the `Balances` pallet to do the transfer here, the account balance
		// is stored (and can be observed) in the `System` pallet's storage.
		let _ = Balances::deposit(
			&AccountId::from(hex!(
				"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
			)),
			token_amount.0.into(),
			Precision::Exact,
		)?;
		log::info!("💸 Registered transfer of {} native tokens", token_amount.0);
		Ok(())
	}
}

impl pallet_native_token_management::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type TokenTransferHandler = TokenTransferHandler;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxValidators;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type SlotDuration = ConstU64<SLOT_DURATION>;
}

pallet_session_runtime_stub::impl_pallet_session_config!(Runtime);

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
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
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
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = RuntimeFreezeReason;
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
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = weights::pallet_sudo::WeightInfo<Runtime>;
}

impl pallet_partner_chains_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ShouldEndSession = ValidatorManagementSessionManager<Runtime>;
	type NextSessionRotation = ();
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

	fn select_authorities(
		input: AuthoritySelectionInputs,
		sidechain_epoch: ScEpochNumber,
	) -> Option<BoundedVec<(Self::AuthorityId, Self::AuthorityKeys), Self::MaxValidators>> {
		select_authorities(Sidechain::sidechain_params(), input, sidechain_epoch)
	}

	fn current_epoch_number() -> ScEpochNumber {
		Sidechain::current_epoch_number()
	}

	type WeightInfo = weights::pallet_session_validator_management::WeightInfo<Runtime>;
}

parameter_types! {
	pub const TokenConversionRate: u128 = 1_000_000_000u128;
	pub const MaxTransactions: u32 = 256u32;
}

pub struct LogBeneficiaries;
impl sp_sidechain::OnNewEpoch for LogBeneficiaries {
	fn on_new_epoch(old_epoch: ScEpochNumber, _new_epoch: ScEpochNumber) -> sp_weights::Weight {
		let rewards = BlockRewards::get_rewards_and_clear();
		log::info!("Rewards accrued in epoch {old_epoch}: {rewards:?}");

		DbWeight::get().reads_writes(1, 1)
	}
}

impl pallet_sidechain::Config for Runtime {
	fn current_slot_number() -> ScSlotNumber {
		ScSlotNumber(*pallet_aura::CurrentSlot::<Self>::get())
	}
	type OnNewEpoch = LogBeneficiaries;

	type SidechainParams = chain_params::SidechainParams;
}

pub type BeneficiaryId = sidechain_domain::byte_string::SizedByteString<32>;

impl pallet_block_rewards::Config for Runtime {
	type BeneficiaryId = BeneficiaryId;
	type BlockRewardPoints = u32;
	type GetBlockRewardPoints = sp_block_rewards::SimpleBlockCount;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime {
		System: frame_system,
		Timestamp: pallet_timestamp,
		Aura: pallet_aura,
		Grandpa: pallet_grandpa,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
		Sudo: pallet_sudo,
		// Custom Pallets
		// Sidechain pallet must come after the Aura pallet, since it gets the slot number from it
		Sidechain: pallet_sidechain,
		SessionCommitteeManagement: pallet_session_validator_management,
		BlockRewards: pallet_block_rewards,
		// pallet_grandpa reads pallet_session::pallet::CurrentIndex storage.
		// Only stub implementation of pallet_session should be wired.
		// Partner Chains session_manager ValidatorManagementSessionManager writes to pallet_session::pallet::CurrentIndex.
		// ValidatorManagementSessionManager is wired in by pallet_partner_chains_session.
		PalletSession: pallet_session,
		// The order matters!! pallet_partner_chains_session needs to come last for correct initialization order
		Session: pallet_partner_chains_session,
		NativeTokenManagement: pallet_native_token_management,
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
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_sudo, Sudo]
		[pallet_session_validator_management, SessionValidatorManagementBench::<Runtime>]
	);
}

impl_runtime_apis! {
	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, |_| None)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			vec![]
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
			use pallet_session_validator_management_benchmarking::Pallet as SessionValidatorManagementBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch};
			use sp_storage::TrackedStorageKey;

			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;
			use pallet_session_validator_management_benchmarking::Pallet as SessionValidatorManagementBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}
			impl pallet_session_validator_management_benchmarking::Config for Runtime {}

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

	impl sp_sidechain::GetSidechainParams<Block, SidechainParams> for Runtime {
		fn sidechain_params() -> SidechainParams {
			Sidechain::sidechain_params()
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

	impl sp_session_validator_management::SessionValidatorManagementApi<Block, SessionKeys, CrossChainPublic, AuthoritySelectionInputs, sidechain_domain::ScEpochNumber> for Runtime {
		fn get_current_committee() -> (ScEpochNumber, Vec<CrossChainPublic>) {
			SessionCommitteeManagement::get_current_committee()
		}
		fn get_next_committee() -> Option<(ScEpochNumber, Vec<CrossChainPublic>)> {
			SessionCommitteeManagement::get_next_committee()
		}
		fn get_next_unset_epoch_number() -> sidechain_domain::ScEpochNumber {
			SessionCommitteeManagement::get_next_unset_epoch_number()
		}
		fn calculate_committee(authority_selection_inputs: AuthoritySelectionInputs, sidechain_epoch: ScEpochNumber) -> Option<Vec<(CrossChainPublic, SessionKeys)>> {
			SessionCommitteeManagement::calculate_committee(authority_selection_inputs, sidechain_epoch)
		}
		fn get_main_chain_scripts() -> sp_session_validator_management::MainChainScripts {
			SessionCommitteeManagement::get_main_chain_scripts()
		}
	}

	impl authority_selection_inherents::filter_invalid_candidates::CandidateValidationApi<Block> for Runtime {
		fn validate_registered_candidate_data(mainchain_pub_key: &MainchainPublicKey, registration_data: &RegistrationData) -> Option<RegistrationDataError> {
			authority_selection_inherents::filter_invalid_candidates::validate_registration_data(mainchain_pub_key, registration_data, &Sidechain::sidechain_params()).err()
		}
		fn validate_stake(stake: Option<StakeDelegation>) -> Option<StakeError> {
			authority_selection_inherents::filter_invalid_candidates::validate_stake(stake).err()
		}
		fn validate_permissioned_candidate_data(candidate: PermissionedCandidateData) -> Option<PermissionedCandidateDataError> {
			validate_permissioned_candidate_data::<CrossChainPublic>(candidate).err()
		}
	}

	impl sp_native_token_management::NativeTokenManagementApi<Block> for Runtime {
		fn get_main_chain_scripts() -> sp_native_token_management::MainChainScripts {
			NativeTokenManagement::get_main_chain_scripts()
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
