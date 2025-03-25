use derive_new::new;
use jsonrpsee::{
	core::{async_trait, RpcResult},
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

#[rpc(client, server, namespace = "block-producer-metadata")]
pub trait BlockProducerMetadataRpcApi<Metadata> {
	#[method(name = "getMetadata")]
	fn get_metadata(
		&self,
		cross_chain_public_key: CrossChainPublicKey,
	) -> RpcResult<Option<Metadata>>;
}

#[derive(new)]
pub struct BlockProducerMetadataRpc<C, Block, Metadata> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<Block>,
	_marker1: std::marker::PhantomData<Metadata>,
}

impl<C, B, Metadata> BlockProducerMetadataRpc<C, B, Metadata> {}

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
	fn get_metadata(
		&self,
		cross_chain_public_key: CrossChainPublicKey,
	) -> RpcResult<Option<Metadata>> {
		let api = self.client.runtime_api();
		let best_block = self.client.info().best_hash;
		api.get_metadata_for(best_block, &cross_chain_public_key)
			.map_err(error_object_from)
	}
}

pub fn error_object_from<T: std::fmt::Debug>(err: T) -> ErrorObjectOwned {
	ErrorObject::owned::<u8>(-1, format!("{err:?}"), None)
}
