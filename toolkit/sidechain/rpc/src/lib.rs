pub mod types;

use derive_new::new;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
	types::{error::ErrorCode, ErrorObject, ErrorObjectOwned},
};
use sidechain_domain::mainchain_epoch::{MainchainEpochConfig, MainchainEpochDerivation};
use sidechain_domain::MainchainBlock;
use sidechain_slots::SlotApi;
use sp_api::ProvideRuntimeApi;
use sp_core::offchain::Timestamp;
use sp_runtime::traits::Block as BlockT;
use sp_sidechain::{GetGenesisUtxo, GetSidechainStatus};
use std::sync::Arc;
use time_source::*;
use types::*;

#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "mock"))]
pub mod mock;

#[rpc(client, server, namespace = "sidechain")]
pub trait SidechainRpcApi {
	#[method(name = "getParams")]
	fn get_params(&self) -> RpcResult<sp_sidechain::query::Output>;

	/// Returns data related to the status of both the main chain and the sidechain, like their epochs or the timestamp associated to the next epoch.
	#[method(name = "getStatus")]
	async fn get_status(&self) -> RpcResult<GetStatusResponse>;
}

#[async_trait]
pub trait SidechainRpcDataSource {
	async fn get_latest_block_info(
		&self,
	) -> Result<MainchainBlock, Box<dyn std::error::Error + Send + Sync>>;
}

#[derive(new)]
pub struct SidechainRpc<C, Block> {
	client: Arc<C>,
	mc_epoch_config: MainchainEpochConfig,
	data_source: Arc<dyn SidechainRpcDataSource + Send + Sync>,
	time_source: Arc<dyn TimeSource + Send + Sync>,
	_marker: std::marker::PhantomData<Block>,
}

impl<C, B> SidechainRpc<C, B> {
	fn get_current_timestamp(&self) -> Timestamp {
		Timestamp::from_unix_millis(self.time_source.get_current_time_millis())
	}
}

#[async_trait]
impl<C, Block> SidechainRpcApiServer for SidechainRpc<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: GetBestHash<Block>,
	C::Api: SlotApi<Block> + GetGenesisUtxo<Block> + GetSidechainStatus<Block>,
{
	fn get_params(&self) -> RpcResult<sp_sidechain::query::Output> {
		let api = self.client.runtime_api();
		let best_block = self.client.best_hash();

		let genesis_utxo = api.genesis_utxo(best_block).map_err(error_object_from)?;

		Ok(sp_sidechain::query::Output { genesis_utxo })
	}

	async fn get_status(&self) -> RpcResult<GetStatusResponse> {
		let api = self.client.runtime_api();
		let best_block = self.client.best_hash();

		let slot_config = api.slot_config(best_block).map_err(error_object_from)?;

		let current_timestamp = self.get_current_timestamp();
		let current_sidechain_slot =
			slot_config.slot_from_timestamp(current_timestamp.unix_millis());
		let current_sidechain_epoch = slot_config.epoch_number(current_sidechain_slot);
		let next_sidechain_epoch_timestamp = slot_config
			.epoch_start_time(current_sidechain_epoch.next())
			.ok_or(GetStatusRpcError::CannotConvertSidechainSlotToTimestamp)?;

		let latest_mainchain_block =
			self.data_source.get_latest_block_info().await.map_err(|err| {
				ErrorObject::owned(
					ErrorCode::InternalError.code(),
					format!("Internal error: GetLatestBlockResponse error '{:?}", err),
					None::<u8>,
				)
			})?;
		let next_mainchain_epoch_timestamp = self
			.mc_epoch_config
			.mainchain_epoch_to_timestamp(latest_mainchain_block.epoch.next());

		Ok(GetStatusResponse {
			sidechain: SidechainData {
				epoch: current_sidechain_epoch.0,
				slot: current_sidechain_slot.into(),
				next_epoch_timestamp: next_sidechain_epoch_timestamp,
			},
			mainchain: MainchainData {
				epoch: latest_mainchain_block.epoch.0,
				slot: latest_mainchain_block.slot.0,
				next_epoch_timestamp: next_mainchain_epoch_timestamp,
			},
		})
	}
}

pub fn error_object_from<T: std::fmt::Debug>(err: T) -> ErrorObjectOwned {
	ErrorObject::owned::<u8>(-1, format!("{err:?}"), None)
}
