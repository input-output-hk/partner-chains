use super::*;
use frame_benchmarking::v2::*;
use frame_support::BoundedVec;
use frame_support::assert_ok;
use frame_support::traits::Get;
use frame_system::RawOrigin;
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

impl<T: crate::Config> BenchmarkHelper<T> for () {
	fn transfers(_t: u32) -> BoundedVec<BridgeTransferV1<T::Recipient>, T::MaxTransfersPerBlock> {
		BoundedVec::new()
	}

	fn data_checkpoint() -> BridgeDataCheckpoint {
		Default::default()
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn handle_transfers(
		t: Linear<1, { T::MaxTransfersPerBlock::get() }>,
		s: Linear<1, { T::MaxTransfersPerBlock::get() }>,
	) {
		assert_ok!(Pallet::<T>::set_main_chain_scripts(
			RawOrigin::Root.into(),
			T::BenchmarkHelper::main_chain_scripts(),
		));

		let transfers = T::BenchmarkHelper::transfers(t);
		let data_checkpoint = T::BenchmarkHelper::data_checkpoint();

		#[extrinsic_call]
		_(RawOrigin::None, transfers, data_checkpoint);
	}

	#[benchmark]
	fn set_main_chain_scripts() {
		let new_main_chain_scripts = T::BenchmarkHelper::main_chain_scripts();

		#[extrinsic_call]
		_(RawOrigin::Root, new_main_chain_scripts);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
