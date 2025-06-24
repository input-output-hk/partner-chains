use frame_support::parameter_types;
use frame_support::traits::ConstU32;
use frame_support::{
	construct_runtime,
	traits::{ConstU16, ConstU64},
};
use hex_literal::hex;
use sidechain_domain::*;
use sp_core::crypto::Ss58Codec;
use sp_core::{ConstU128, H256};
use sp_runtime::{
	AccountId32, BuildStorage,
	traits::{BlakeTwo256, IdentityLookup},
};

pub type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = AccountId32;
pub type PartnerChainAddress = AccountId32;

#[frame_support::pallet]
pub mod mock_pallet {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	pub type LastNewAssociation<T: Config> =
		StorageValue<_, (PartnerChainAddress, MainchainKeyHash), OptionQuery>;

	impl<T: Config> crate::OnNewAssociation<AccountId> for Pallet<T> {
		fn on_new_association(
			partner_chain_address: PartnerChainAddress,
			main_chain_key_hash: MainchainKeyHash,
		) {
			LastNewAssociation::<T>::put((partner_chain_address, main_chain_key_hash));
		}
	}
}

construct_runtime! {
	pub enum Test {
		System: frame_system,
		AddressAssociations: crate::pallet,
		Balances: pallet_balances,
		MockPallet: mock_pallet
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
	type AccountData = pallet_balances::AccountData<u128>;
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
	type Balance = u128;
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
	pub const AssociationFeeBurn: u128 = 1000;
}

pub(crate) const FUNDED_ACCOUNT: AccountId32 = AccountId32::new([1; 32]);

pub(crate) const STAKE_PUBLIC_KEY: StakePublicKey =
	StakePublicKey(hex!("2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c"));

pub(crate) fn pc_address() -> AccountId32 {
	AccountId32::from_ss58check("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY").unwrap()
}

pub(crate) const VALID_SIGNATURE: [u8; 64] = hex!(
	"36aabd5267699b01c01fb6892f9888ab82a0b853a21dcd863b8241c3049d85163ddf350cbbc8ba724abe7b22d5ae03a7b1429f4cc37fb11afcce041fac1cdd05"
);

impl crate::pallet::Config for Test {
	type WeightInfo = ();
	type PartnerChainAddress = PartnerChainAddress;
	fn genesis_utxo() -> UtxoId {
		UtxoId::new(hex!("0000000000000000000000000000000000000000000000000000000000000000"), 0)
	}
	type Currency = Balances;
	type BurnAmount = AssociationFeeBurn;
	type OnNewAssociation = MockPallet;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(FUNDED_ACCOUNT, 100_000)],
		dev_accounts: None,
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}
