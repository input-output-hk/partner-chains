use crate::Result;
use sidechain_domain::*;
use sp_partner_chains_bridge::{
	BridgeDataCheckpoint, BridgeTransferV1, MainChainScripts, TokenBridgeDataSource,
};
use std::marker::PhantomData;

pub struct TokenBridgeDataSourceImpl<RecipientAddress> {
	_phantom: PhantomData<RecipientAddress>,
}

impl<RecipientAddress> TokenBridgeDataSourceImpl<RecipientAddress> {
	pub fn new() -> Self {
		Self { _phantom: PhantomData::default() }
	}
}

#[async_trait::async_trait]
impl<RecipientAddress: Send + Sync> TokenBridgeDataSource<RecipientAddress>
	for TokenBridgeDataSourceImpl<RecipientAddress>
{
	async fn get_transfers(
		&self,
		_main_chain_scripts: MainChainScripts,
		_data_checkpoint: BridgeDataCheckpoint,
		_max_transfers: u32,
		_current_mc_block: McBlockHash,
	) -> Result<(Vec<BridgeTransferV1<RecipientAddress>>, BridgeDataCheckpoint)> {
		Err("not implemented".into())
	}
}
