//! Json RPC for the Sidechain pallet
//!
//! # Usage
//!
//! ## Implementing runtime APIs
//!
//! Your runtime should implement the [GetGenesisUtxo] and [GetEpochDurationApi] runtime APIs.
//! For example, if your chain uses Aura for consensus, they may be implemented similar to this:
//! ```rust,ignore
//! impl sp_sidechain::GetGenesisUtxo<Block> for Runtime {
//!   fn genesis_utxo() -> UtxoId {
//!     Sidechain::genesis_utxo()
//!   }
//! }
//!
//! impl sp_sidechain::GetEpochDurationApi<Block> for Runtime {
//! 	fn get_epoch_duration_millis() -> u64 {
//! 		BLOCKS_PER_EPOCH * MILLISECS_PER_BLOCK
//! 	}
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
//! # use sp_api::{ CallApiAt, ProvideRuntimeApi };
//! # use sp_runtime::traits::Block as BlockT;
//! # use sp_sidechain::{ GetEpochDurationApi, GetGenesisUtxo };
//! # use std::sync::Arc;
//! # use time_source::TimeSource;
//! fn create_rpc<B: BlockT, C: Send + Sync + 'static>(
//!    client: Arc<C>,
//!    time_source: Arc<dyn TimeSource + Send + Sync>,
//!    data_source: Arc<dyn SidechainRpcDataSource + Send + Sync>,
//! ) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
//! where
//!     C: ProvideRuntimeApi<B> + GetBestHash<B> + CallApiAt<B>,
//!     C::Api: GetGenesisUtxo<B> + GetEpochDurationApi<B>
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
//! ## Legacy compatibility mode
//!
//! In previous versions of the Partner Chains toolkit, the RPC services in this crate relied on now
//! deprecated runtime api `sp_sidechain::SlotApi` to compute Partner Chain epochs and current slot.
//! For Partner Chains that started before this dependency was removed, a compatibility mode is
//! provided behind the `legacy-slotapi-compat` feature flag. Enabling this flag will cause the nodes to
//! use this runtime API when present and optionally include information about current Patner Chain
//! slot in `sidechain.slot` field of `sidechain_getStatus` response.
//!
//! [GetGenesisUtxo]: sp_sidechain::GetGenesisUtxo
//! [GetEpochDurationApi]: sp_sidechain::GetEpochDurationApi
#![deny(missing_docs)]
use derive_new::new;
use jsonrpsee::{
	core::{RpcResult, async_trait},
	proc_macros::rpc,
	types::{ErrorObject, ErrorObjectOwned, error::ErrorCode},
};
#[cfg(feature = "legacy-slotapi-compat")]
use legacy_compat::slots::*;
use sidechain_domain::mainchain_epoch::{MainchainEpochConfig, MainchainEpochDerivation};
use sidechain_domain::{MainchainBlock, UtxoId};
use sp_api::ProvideRuntimeApi;
use sp_core::offchain::{Duration, Timestamp};
use sp_runtime::traits::Block as BlockT;
use sp_sidechain::{GetEpochDurationApi, GetGenesisUtxo};
use std::sync::Arc;
use time_source::*;
use types::*;

/// Response types returned by RPC endpoints for Sidechain pallet
pub mod types;

#[cfg(feature = "legacy-slotapi-compat")]
mod legacy_compat;

#[cfg(test)]
mod tests;

#[cfg(any(test))]
mod mock;

/// Response type of the [crate::SidechainRpcApiServer::get_params] method
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

/// Runtime client that can serve data to sidechain RPC
trait SidechainRpcClient<Block: BlockT> {
	/// Returns Partner Chain epoch duration
	fn get_epoch_duration(
		&self,
		best_block: Block::Hash,
	) -> Result<Duration, Box<dyn std::error::Error + Send + Sync>>;

	/// Returns the Partner Chain's genesis UTXO
	fn get_genesis_utxo(
		&self,
		best_block: Block::Hash,
	) -> Result<UtxoId, Box<dyn std::error::Error + Send + Sync>>;

	#[cfg(feature = "legacy-slotapi-compat")]
	/// Returns slot duration
	fn get_maybe_slot_duration(&self, best_block: Block::Hash) -> Option<u64>;
}

impl<Block, Client> SidechainRpcClient<Block> for Client
where
	Block: BlockT,
	Client: sp_api::CallApiAt<Block> + Send + Sync + 'static,
	Client: ProvideRuntimeApi<Block>,
	Client::Api: GetEpochDurationApi<Block>,
	Client::Api: GetGenesisUtxo<Block>,
{
	fn get_epoch_duration(
		&self,
		best_block: Block::Hash,
	) -> Result<Duration, Box<dyn std::error::Error + Send + Sync>> {
		#[cfg(feature = "legacy-slotapi-compat")]
		if let Some(slot_config) = self.get_sc_slot_config(best_block) {
			return Ok(Duration::from_millis(
				u64::from(slot_config.slots_per_epoch) * slot_config.slot_duration_millis,
			));
		}

		Ok(Duration::from_millis(self.runtime_api().get_epoch_duration_millis(best_block)?))
	}

	fn get_genesis_utxo(
		&self,
		best_block: Block::Hash,
	) -> Result<UtxoId, Box<dyn std::error::Error + Send + Sync>> {
		Ok(self.runtime_api().genesis_utxo(best_block)?)
	}

	#[cfg(feature = "legacy-slotapi-compat")]
	fn get_maybe_slot_duration(&self, best_block: Block::Hash) -> Option<u64> {
		let slot_config = self.get_sc_slot_config(best_block)?;
		Some(slot_config.slot_duration_millis)
	}
}

#[async_trait]
impl<C, Block> SidechainRpcApiServer for SidechainRpc<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static,
	C: SidechainRpcClient<Block>,
	C: GetBestHash<Block>,
{
	fn get_params(&self) -> RpcResult<GetParamsOutput> {
		let best_block = self.client.best_hash();

		let genesis_utxo = self.client.get_genesis_utxo(best_block).map_err(error_object_from)?;

		Ok(GetParamsOutput { genesis_utxo })
	}

	async fn get_status(&self) -> RpcResult<GetStatusResponse> {
		let best_block = self.client.best_hash();

		let current_timestamp = self.get_current_timestamp();

		let sc_epoch_duration: Duration =
			self.client.get_epoch_duration(best_block).map_err(error_object_from)?;

		let current_sidechain_epoch = current_timestamp.unix_millis() / sc_epoch_duration.millis();

		let next_sidechain_epoch_timestamp =
			Timestamp::from((current_sidechain_epoch + 1) * sc_epoch_duration.millis());

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
				epoch: current_sidechain_epoch,
				#[cfg(feature = "legacy-slotapi-compat")]
				slot: (self.client.get_maybe_slot_duration(best_block))
					.map(|duration| current_timestamp.unix_millis() / duration),
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
