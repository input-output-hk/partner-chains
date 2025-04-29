//! Crate providing Json RPC methods for the Block Producer Metadata feature of the Partner Chains Toolkit
//!
//! ## Contents
//!
//! This crate provides the [BlockProducerMetadataRpcApiServer] trait defining the JsonRPC methods related to
//! block producer metadata and its concrete implementation [BlockProducerMetadataRpc]. Currently, only a
//! single method is provided:
//! - `pc_getBlockProducerMetadata`
//!
//! ## Usage - PC Builders
//!
//! To use the Json RPC service defined in this crate, first make your runtime implement
//! [sp_block_producer_metadata::BlockProducerMetadataApi]. Eg. assuming the pallet `BlockProducerMetadataPallet`
//! in your runtime uses `BlockProducerMetadataType` as the metadata type, the following should be included in your
//! `impl_runtime_apis` block:
//! ```rust, ignore
//! impl BlockProducerMetadataApi<Block, BlockProducerMetadataType> for Runtime
//! {
//! 	fn get_metadata_for(
//! 		cross_chain_pub_key: &CrossChainPublicKey,
//! 	) -> Option<BlockProducerMetadataType> {
//! 		BlockProducerMetadataPallet::get_metadata_for(&cross_chain_pub_key)
//! 	}
//! }
//! ```
//!
//! Afterwards, the [BlockProducerMetadataRpc] Json RPC service can be added into the Json RPC stack of your node, eg.:
//!
//! ```rust
//! use jsonrpsee::RpcModule;
//! use std::sync::Arc;
//! use sp_block_producer_metadata::*;
//! use pallet_block_producer_metadata_rpc::*;
//!
//! fn create_rpc<C, Block, Metadata>(client: Arc<C>) -> Result<RpcModule<()>, Box<dyn std::error::Error>>
//! where
//!   C: Send + Sync + 'static,
//!   Block: sp_runtime::traits::Block,
//!   Metadata: Send + Sync + Clone + sp_core::Decode + sp_runtime::Serialize + 'static,
//!   C: sp_api::ProvideRuntimeApi<Block>,
//!   C: sp_blockchain::HeaderBackend<Block>,
//!   C::Api: BlockProducerMetadataApi<Block, Metadata>
//! {
//!     let mut module = RpcModule::new(());
//!     module.merge(BlockProducerMetadataRpc::new(client.clone()).into_rpc())?;
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
use sidechain_domain::CrossChainPublicKey;
use sp_api::ProvideRuntimeApi;
use sp_block_producer_metadata::BlockProducerMetadataApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

/// Json RPC methods related to the Block Producer Metadata feature of Partner Chains Toolkit
#[rpc(client, server, namespace = "pc")]
pub trait BlockProducerMetadataRpcApi<Metadata> {
	/// Returns JSON-encoded metadata for given `cross_chain_public_key` from the storage of the current tip if it exists.
	#[method(name = "getMetadata")]
	fn get_block_producer_metadata(
		&self,
		cross_chain_public_key: CrossChainPublicKey,
	) -> RpcResult<Option<Metadata>>;
}

/// Concrete implementation of [BlockProducerMetadataRpcApiServer] that uses [BlockProducerMetadataApi] for querying runtime storage.
#[derive(new)]
pub struct BlockProducerMetadataRpc<C, Block, Metadata> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<(Block, Metadata)>,
}

#[async_trait]
impl<C, Block, Metadata: Decode + Sync + Send + 'static> BlockProducerMetadataRpcApiServer<Metadata>
	for BlockProducerMetadataRpc<C, Block, Metadata>
where
	Block: BlockT,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: BlockProducerMetadataApi<Block, Metadata>,
{
	fn get_block_producer_metadata(
		&self,
		cross_chain_public_key: CrossChainPublicKey,
	) -> RpcResult<Option<Metadata>> {
		let api = self.client.runtime_api();
		let best_block = self.client.info().best_hash;
		api.get_metadata_for(best_block, &cross_chain_public_key)
			.map_err(error_object_from)
	}
}

fn error_object_from<T: std::fmt::Debug>(err: T) -> ErrorObjectOwned {
	ErrorObject::owned::<u8>(-1, format!("{err:?}"), None)
}
