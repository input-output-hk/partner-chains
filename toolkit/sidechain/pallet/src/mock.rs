use crate::pallet;
use frame_support::{
	sp_runtime::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
	},
	*,
};
use sidechain_domain::{McTxHash, ScEpochDuration, UtxoId, UtxoIndex};
use sp_core::*;

pub const MOCK_GENESIS_UTXO: UtxoId = UtxoId { tx_hash: McTxHash([0u8; 32]), index: UtxoIndex(0) };

pub(crate) const EPOCH_DURATION_MILLIS: u64 = 36000;

#[frame_support::pallet]
pub(crate) mod mock_pallet {
	use frame_support::pallet_prelude::*;
	use sidechain_domain::ScEpochNumber;

	use sp_sidechain::OnNewEpoch;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	pub type CurrentEpoch<T: Config> = StorageValue<_, ScEpochNumber, ValueQuery>;

	#[pallet::storage]
	pub type OnNewEpochCallCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	impl<T: Config> OnNewEpoch for Pallet<T> {
		fn on_new_epoch(_old_epoch: ScEpochNumber, _new_epoch: ScEpochNumber) -> Weight {
			OnNewEpochCallCount::<T>::mutate(|c| *c += 1);
			Weight::zero()
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn set_epoch(epoch: u64) {
			CurrentEpoch::<T>::put(ScEpochNumber(epoch))
		}
	}
}

construct_runtime! {
	pub enum Test {
		System: frame_system,
		Sidechain: pallet,
		Mock: mock_pallet
	}
}
impl mock_pallet::Config for Test {}

pub type Block = frame_system::mocking::MockBlock<Test>;

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
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

impl pallet::Config for Test {
	fn reference_timestamp_millis() -> u64 {
		mock_pallet::CurrentEpoch::<Test>::get().0 * EPOCH_DURATION_MILLIS
	}
	type OnNewEpoch = Mock;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet::GenesisConfig::<Test> {
		genesis_utxo: MOCK_GENESIS_UTXO,
		epoch_duration: ScEpochDuration::from_millis(EPOCH_DURATION_MILLIS),
		_config: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}
