//! Json RPC for the Block Producer Fees pallet
//!
//! ## Contents
//!
//! This crate provides the [BlockProducerFeesRpcApiServer] trait defining the JsonRPC method to display
//! block producer fees and its concrete implementation [BlockProducerFeesRpc].
//! ## Usage - PC Builders
//!
//! To use the Json RPC service defined in this crate, first make your runtime implement
//! [sp_block_producer_fees::BlockProducerFeesApi]. Eg. assuming the pallet `BlockProducerFeesPallet`
//! in your runtime uses `AccountId` as the account id type, the following should be included in your
//! `impl_runtime_apis` block:
//! ```rust, ignore
//! impl BlockProducerFeesApi<Block, AccountId> for Runtime
//! {
//! 	fn get_all_fees() -> Vec<(AccountId, sp_block_producer_fees::PerTenThousands)> {
//!			BlockProducerFees::get_all_latest().map(|(account_id, (_slot, fee))| (account_id, fee)).collect()
//!		}
//! }
//! ```
//!
//! Afterwards, the [BlockProducerFeesRpc] Json RPC service can be added into the Json RPC stack of your node.
//! Example where AccountId type parameter is set to AccountId32:
//!
//! ```rust
//! use jsonrpsee::RpcModule;
//! use std::sync::Arc;
//! use sp_block_producer_fees::*;
//! use pallet_block_producer_fees_rpc::*;
//!
//! fn create_rpc<C, Block>(client: Arc<C>) -> Result<RpcModule<()>, Box<dyn std::error::Error>>
//! where
//!   C: Send + Sync + 'static,
//!   Block: sp_runtime::traits::Block,
//!   C: sp_api::ProvideRuntimeApi<Block>,
//!   C: sp_blockchain::HeaderBackend<Block>,
//!   C::Api: BlockProducerFeesApi<Block, sp_runtime::AccountId32>
//! {
//!     let mut module = RpcModule::new(());
//!     module.merge(BlockProducerFeesRpc::new(client.clone()).into_rpc())?;
//!     // other RPC modules
//!     Ok(module)
//! }
//! ```
#![deny(missing_docs)]
use derive_new::new;
use jsonrpsee::{
	core::{RpcResult, async_trait},
	proc_macros::rpc,
	types::{ErrorObject, ErrorObjectOwned},
};
use parity_scale_codec::Decode;
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_block_producer_fees::BlockProducerFeesApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

/// Json RPC methods related to the Block Producer Metadata feature of Partner Chains Toolkit
#[rpc(client, server, namespace = "pc")]
pub trait BlockProducerFeesRpc<AccountId: Decode> {
	/// Returns the latest recorded fees. To get all stored data query pallet storage directly.
	#[method(name = "getBlockProducerFees")]
	fn get_block_producer_fees(&self) -> RpcResult<Vec<FeesSettings<AccountId>>>;
}

/// Concrete implementation of [BlockProducerFeesRpcApiServer] that uses [BlockProducerFeesRpcApi] for querying runtime storage.
#[derive(new)]
pub struct BlockProducerFeesRpc<C, Block, AccountId> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<(Block, AccountId)>,
}

/// Fees settings for given account
#[derive(Clone, Deserialize, Serialize)]
pub struct FeesSettings<AccountId> {
	account_id: AccountId,
	/// Margin fee in percent
	margin_fee: f64,
}

#[async_trait]
impl<C, Block, AccountId: Decode + Sync + Send + 'static> BlockProducerFeesRpcServer<AccountId>
	for BlockProducerFeesRpc<C, Block, AccountId>
where
	Block: BlockT,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: BlockProducerFeesApi<Block, AccountId>,
{
	fn get_block_producer_fees(&self) -> RpcResult<Vec<FeesSettings<AccountId>>> {
		let api = self.client.runtime_api();
		let best_block = self.client.info().best_hash;
		let fees = api.get_all_fees(best_block).map_err(error_object_from)?;
		Ok(fees
			.into_iter()
			.map(|(account_id, fee)| {
				let fee: f64 = fee.into();
				FeesSettings { account_id, margin_fee: fee / 100f64 }
			})
			.collect())
	}
}

fn error_object_from<T: std::fmt::Debug>(err: T) -> ErrorObjectOwned {
	ErrorObject::owned::<u8>(-1, format!("{err:?}"), None)
}
