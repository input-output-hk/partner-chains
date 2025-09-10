use crate::Result;
use sidechain_domain::*;
use sp_partner_chains_bridge::{
	BridgeDataCheckpoint, BridgeTransferV1, MainChainScripts, TokenBridgeDataSource,
};
use std::marker::PhantomData;
/// Mocked token bridge data source
pub struct TokenBridgeDataSourceMock<RecipientAddress> {
	_phantom: PhantomData<RecipientAddress>,
}

impl<RecipientAddress> TokenBridgeDataSourceMock<RecipientAddress> {
	/// Creates a new mocked token bridge data source
	pub fn new() -> Self {
		Self { _phantom: Default::default() }
	}
}

#[async_trait::async_trait]
impl<RecipientAddress: Send + Sync> TokenBridgeDataSource<RecipientAddress>
	for TokenBridgeDataSourceMock<RecipientAddress>
{
	async fn get_transfers(
		&self,
		_main_chain_scripts: MainChainScripts,
		_data_checkpoint: BridgeDataCheckpoint,
		_max_transfers: u32,
		_current_mc_block: McBlockHash,
	) -> Result<(Vec<BridgeTransferV1<RecipientAddress>>, BridgeDataCheckpoint)> {
		Ok((vec![], BridgeDataCheckpoint::Block(McBlockNumber(0))))
	}
}
