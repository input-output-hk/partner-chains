use frame_support::traits::{ConstU32, ConstU128};
use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU16, ConstU64},
};
use hex_literal::hex;
use scale_info::TypeInfo;
use sidechain_domain::byte_string::{BoundedString, SizedByteString};
use sidechain_domain::*;
use sp_block_producer_metadata::MetadataSignedMessage;
use sp_core::H256;
use sp_runtime::codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use sp_runtime::{
	AccountId32, BuildStorage,
	traits::{BlakeTwo256, IdentityLookup},
};

pub type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = AccountId32;
pub type Balance = u128;

#[frame_support::pallet]
pub mod mock_pallet {
	use frame_support::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	pub type CurrentTime<T: Config> = StorageValue<_, u64, ValueQuery>;

	impl<T: Config> Pallet<T> {
		pub fn current_time() -> u64 {
			CurrentTime::<T>::get()
		}
	}
}

construct_runtime! {
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		BlockProducerMetadata: crate::pallet,
		Mock: mock_pallet
	}
}

impl mock_pallet::Config for Test {}

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
	type AccountData = pallet_balances::AccountData<Balance>;
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

impl pallet_balances::Config for Test {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = ();
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const MetadataHoldAmount: Balance = 1000;
}

#[derive(
	Clone, Debug, MaxEncodedLen, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo,
)]
pub struct BlockProducerUrlMetadata {
	pub url: BoundedString<ConstU32<512>>,
	pub hash: SizedByteString<32>,
}

#[cfg(feature = "runtime-benchmarks")]
pub struct PalletBlockProducerMetadataBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl crate::benchmarking::BenchmarkHelper<BlockProducerUrlMetadata, AccountId32>
	for PalletBlockProducerMetadataBenchmarkHelper
{
	fn genesis_utxo() -> UtxoId {
		genesis_utxo()
	}

	fn metadata() -> BlockProducerUrlMetadata {
		BlockProducerUrlMetadata {
			url: "https://cool.stuff/spo.json".try_into().unwrap(),
			hash: SizedByteString::from([0; 32]),
		}
	}

	fn cross_chain_pub_key() -> CrossChainPublicKey {
		cross_chain_pub_key()
	}

	fn cross_chain_sign_key() -> k256::SecretKey {
		cross_chain_priv_key()
	}

	fn upsert_valid_before() -> u64 {
		100_000_000
	}

	fn delete_valid_before() -> u64 {
		100_000_000
	}
}

pub(crate) const FUNDED_ACCOUNT: AccountId32 = AccountId32::new([1; 32]);
pub(crate) const FUNDED_ACCOUNT_2: AccountId32 = AccountId32::new([2; 32]);
pub(crate) const POOR_ACCOUNT: AccountId32 = AccountId32::new([3; 32]);

pub(crate) const INITIAL_BALANCE: u128 = 100_000;

pub(crate) fn genesis_utxo() -> UtxoId {
	UtxoId::new(hex!("59104061ffa0d66f9ba0135d6fc6a884a395b10f8ae9cb276fc2c3bfdfedc260"), 1)
}

impl crate::pallet::Config for Test {
	type WeightInfo = ();
	type BlockProducerMetadata = BlockProducerUrlMetadata;
	fn genesis_utxo() -> UtxoId {
		genesis_utxo()
	}
	fn current_time() -> u64 {
		Mock::current_time()
	}
	type Currency = Balances;
	type HoldAmount = MetadataHoldAmount;
	type RuntimeHoldReason = RuntimeHoldReason;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = PalletBlockProducerMetadataBenchmarkHelper;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(FUNDED_ACCOUNT, INITIAL_BALANCE), (FUNDED_ACCOUNT_2, INITIAL_BALANCE)],
		dev_accounts: None,
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}

pub(crate) fn cross_chain_pub_key() -> CrossChainPublicKey {
	// pub key of secret key cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854
	CrossChainPublicKey(
		hex!("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").to_vec(),
	)
}

pub(crate) fn cross_chain_priv_key() -> k256::SecretKey {
	k256::SecretKey::from_slice(&hex!(
		"cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"
	))
	.unwrap()
}

pub(crate) fn valid_before() -> u64 {
	100_000_000
}

pub(crate) fn cross_chain_signature(
	owner: AccountId32,
	metadata: Option<BlockProducerUrlMetadata>,
) -> CrossChainSignature {
	MetadataSignedMessage {
		cross_chain_pub_key: cross_chain_pub_key(),
		metadata,
		genesis_utxo: genesis_utxo(),
		valid_before: valid_before(),
		owner,
	}
	.sign_with_key(&cross_chain_priv_key())
}
