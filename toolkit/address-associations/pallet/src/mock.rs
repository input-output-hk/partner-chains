use frame_support::traits::ConstU32;
use frame_support::{
	construct_runtime,
	traits::{ConstU16, ConstU64},
};
use hex_literal::hex;
use sidechain_domain::*;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, BuildStorage,
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
			LastNewAssociation::<T>::put((partner_chain_address, main_chain_key_hash))
		}
	}
}

construct_runtime! {
	pub enum Test {
		System: frame_system,
		AddressAssociations: crate::pallet,
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

impl crate::pallet::Config for Test {
	type WeightInfo = ();
	type PartnerChainAddress = PartnerChainAddress;
	fn genesis_utxo() -> UtxoId {
		UtxoId::new(hex!("59104061ffa0d66f9ba0135d6fc6a884a395b10f8ae9cb276fc2c3bfdfedc260"), 1)
	}

	type OnNewAssociation = MockPallet;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
