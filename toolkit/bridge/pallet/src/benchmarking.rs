use super::*;
use frame_benchmarking::v2::*;
use frame_support::{BoundedVec, assert_ok, traits::Get};
use frame_system::RawOrigin;
use sidechain_domain::{McBlockNumber, UtxoId};
use sp_core::{H256, crypto::UncheckedFrom};
use sp_partner_chains_bridge::*;

/// Helper trait for injecting mock values for use in benchmarks
pub trait BenchmarkHelper<T: crate::Config> {
	/// Should return a main chain scripts object
	fn main_chain_scripts() -> MainChainScripts {
		Default::default()
	}

	/// Should return [BoundedVec] of token transfers of length `t`
	fn transfers(t: u32) -> BoundedVec<BridgeTransferV1<T::Recipient>, T::MaxTransfersPerBlock>;

	/// Should return a [BridgeDataCheckpoint]
	fn data_checkpoint() -> BridgeDataCheckpoint;
}

impl<T: crate::Config> BenchmarkHelper<T> for ()
where
	T::Recipient: UncheckedFrom<H256>,
{
	fn transfers(t: u32) -> BoundedVec<BridgeTransferV1<T::Recipient>, T::MaxTransfersPerBlock> {
		use BridgeTransferV1::*;

		let recipient = T::Recipient::unchecked_from(Default::default());
		let utxo_id = UtxoId::default();

		let transfers = alloc::vec![
			UserTransfer { token_amount: 1000, recipient },
			ReserveTransfer { token_amount: 1000 },
			InvalidTransfer { token_amount: 1000, utxo_id },
		]
		.into_iter()
		.cycle()
		.take(t as usize)
		.collect();

		BoundedVec::truncate_from(transfers)
	}

	fn data_checkpoint() -> BridgeDataCheckpoint {
		BridgeDataCheckpoint::Block(McBlockNumber(0))
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn handle_transfers(t: Linear<1, { T::MaxTransfersPerBlock::get() }>) {
		assert_ok!(Pallet::<T>::set_main_chain_scripts(
			RawOrigin::Root.into(),
			T::BenchmarkHelper::main_chain_scripts(),
			T::BenchmarkHelper::data_checkpoint()
		));

		let transfers = T::BenchmarkHelper::transfers(t);
		let data_checkpoint = T::BenchmarkHelper::data_checkpoint();

		#[extrinsic_call]
		_(RawOrigin::None, transfers, data_checkpoint);
	}

	#[benchmark]
	fn set_main_chain_scripts() {
		let new_main_chain_scripts = T::BenchmarkHelper::main_chain_scripts();
		let data_checkpoint = T::BenchmarkHelper::data_checkpoint();

		#[extrinsic_call]
		_(RawOrigin::Root, new_main_chain_scripts, data_checkpoint);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
