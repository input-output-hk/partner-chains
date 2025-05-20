//! Json RPC for the Sidechain pallet
//!
//! # Usage
//!
//! ## Implementing runtime APIs
//!
//! Your runtime should implement the [SlotApi] and [GetGenesisUtxo] runtime APIs. For example, if
//! your chain uses Aura for consensus, they may be implemented similar to this:
//! ```rust,ignore
//! impl sidechain_slots::SlotApi<Block> for Runtime {
//!    fn slot_config() -> sidechain_slots::ScSlotConfig {
//!      sidechain_slots::ScSlotConfig {
//!        slots_per_epoch: Sidechain::slots_per_epoch(),
//!        slot_duration: SlotDuration::from(Aura::slot_duration())
//!      }
//!    }
//!  }
//! impl sp_sidechain::GetGenesisUtxo<Block> for Runtime {
//!   fn genesis_utxo() -> UtxoId {
//!     Sidechain::genesis_utxo()
//!   }
//! }
//! ```
//!
//! ## Adding to the RPC stack
//!
//! Once the runtime APIs are in place, the RPC can be added to the node:
//!
//! ```rust
//! # use jsonrpsee::RpcModule;
//! # use pallet_sidechain_rpc::{*, types::GetBestHash};
//! # use sidechain_domain::mainchain_epoch::MainchainEpochConfig;
//! # use sidechain_slots::SlotApi;
//! # use sp_api::ProvideRuntimeApi;
//! # use sp_runtime::traits::Block as BlockT;
//! # use sp_sidechain::GetGenesisUtxo;
//! # use std::sync::Arc;
//! # use time_source::TimeSource;
//! fn create_rpc<B: BlockT, C: Send + Sync + 'static>(
//!    client: Arc<C>,
//!    time_source: Arc<dyn TimeSource + Send + Sync>,
//!    data_source: Arc<dyn SidechainRpcDataSource + Send + Sync>,
//! ) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
//! where
//!     C: ProvideRuntimeApi<B> + GetBestHash<B>,
//!     C::Api: SlotApi<B> + GetGenesisUtxo<B>
//! {
//!
//!     let mut module = RpcModule::new(());
//!     module.merge(
//!         SidechainRpc::new(
//!             client.clone(),
//!             MainchainEpochConfig::read_from_env().unwrap(),
//!             data_source,
//!             time_source,
//!         )
//!         .into_rpc(),
//!     )?;
//!
//!     // ... other RPCs
//!     Ok(module)
//! }
//! ```
//!
//! Note that your node should already have necessary time and data sources wired in. A Db-Sync-based
//! data source is provided by the Partner Chain toolkit in the `partner_chains_db_sync_data_sources`
//! crate.
//!
//! [GetGenesisUtxo]: sp_sidechain::GetGenesisUtxo
//! [SlotApi]: sidechain_slots::SlotApi
#![deny(missing_docs)]
use derive_new::new;
use jsonrpsee::{
	core::{RpcResult, async_trait},
	proc_macros::rpc,
	types::{ErrorObject, ErrorObjectOwned, error::ErrorCode},
};
use sidechain_domain::mainchain_epoch::{MainchainEpochConfig, MainchainEpochDerivation};
use sidechain_domain::{MainchainBlock, UtxoId};
use sidechain_slots::SlotApi;
use sp_api::ProvideRuntimeApi;
use sp_core::offchain::Timestamp;
use sp_runtime::traits::Block as BlockT;
use sp_sidechain::GetGenesisUtxo;
use std::sync::Arc;
use time_source::*;
use types::*;

/// Response types returned by RPC endpoints for Sidechain pallet
pub mod types;

#[cfg(test)]
mod tests;

#[cfg(any(test))]
mod mock;

/// Response type of the [SidechainRpcApi::get_params] method
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Debug)]
pub struct GetParamsOutput {
	/// Genesis UTXO of the queried Partner Chain
	pub genesis_utxo: UtxoId,
}

/// Json RPC API for querying basic information about a Partner Chain
#[rpc(client, server, namespace = "sidechain")]
pub trait SidechainRpcApi {
	/// Gets the genesis UTXO of the Partner Chain
	///
	/// note: the legacy name `get_params` comes from the times when there were more parameters that
	///       defined a Partner Chain than a single genesis UTXO
	#[method(name = "getParams")]
	fn get_params(&self) -> RpcResult<GetParamsOutput>;

	/// Gets information about current Partner Chain and Cardano slot and epoch number
	#[method(name = "getStatus")]
	async fn get_status(&self) -> RpcResult<GetStatusResponse>;
}

/// Data source used by [SidechainRpc] for querying latest block
#[async_trait]
pub trait SidechainRpcDataSource {
	/// Returns the latest Partner Chain block info
	async fn get_latest_block_info(
		&self,
	) -> Result<MainchainBlock, Box<dyn std::error::Error + Send + Sync>>;
}

/// Json RPC service implementing [SidechainRpcApiServer]
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
	C::Api: SlotApi<Block> + GetGenesisUtxo<Block>,
{
	fn get_params(&self) -> RpcResult<GetParamsOutput> {
		let api = self.client.runtime_api();
		let best_block = self.client.best_hash();

		let genesis_utxo = api.genesis_utxo(best_block).map_err(error_object_from)?;

		Ok(GetParamsOutput { genesis_utxo })
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

fn error_object_from<T: std::fmt::Debug>(err: T) -> ErrorObjectOwned {
	ErrorObject::owned::<u8>(-1, format!("{err:?}"), None)
}
