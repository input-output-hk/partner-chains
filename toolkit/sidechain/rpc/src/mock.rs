use crate::SidechainRpcDataSource;
use derive_new::new;
use jsonrpsee::core::async_trait;
use sidechain_domain::{
	MainchainBlock, McEpochNumber, McSlotNumber, UtxoId, mainchain_epoch::MainchainEpochConfig,
};
use sp_core::offchain::{Duration, Timestamp};
use std::str::FromStr;

// The build.rs file of `substrate_test_runtime` is throwing an error. So a `Block` is being manually defined
pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

#[allow(unused)]
pub(crate) fn mock_utxo_id() -> UtxoId {
	UtxoId::from_str("0000000000000000000000000000000000000000000000000000000000000000#0").unwrap()
}

#[allow(unused)]
pub(crate) fn mock_mc_epoch_config() -> MainchainEpochConfig {
	MainchainEpochConfig {
		first_epoch_timestamp_millis: Timestamp::from_unix_millis(1_000_000_000),
		epoch_duration_millis: Duration::from_millis(120_000),
		first_epoch_number: 50,
		first_slot_number: 501,
		slot_duration_millis: Duration::from_millis(1000),
	}
}

#[allow(unused)]
pub(crate) fn mock_mainchain_block() -> MainchainBlock {
	MainchainBlock { epoch: McEpochNumber(99), slot: McSlotNumber(2000), ..Default::default() }
}

#[derive(new)]
pub struct SidechainRpcDataSourceMock {
	latest_block: MainchainBlock,
}

#[async_trait]
impl SidechainRpcDataSource for SidechainRpcDataSourceMock {
	async fn get_latest_block_info(
		&self,
	) -> Result<MainchainBlock, Box<dyn std::error::Error + Send + Sync>> {
		Ok(self.latest_block.clone())
	}
}
