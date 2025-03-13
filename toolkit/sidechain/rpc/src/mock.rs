use crate::SidechainRpcDataSource;
use derive_new::new;
use jsonrpsee::core::async_trait;
use sidechain_domain::{MainchainBlock, UtxoId};
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
