use crate::GetSidechainParams;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

pub async fn get_sidechain_params<B, C, P>(client: Arc<C>) -> Result<String, String>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: GetSidechainParams<B, P>,
	C: HeaderBackend<B>,
	P: parity_scale_codec::Decode + frame_support::Serialize,
{
	let api = client.runtime_api();
	let best_block = client.info().best_hash;
	let sidechain_params = api.sidechain_params(best_block).map_err(|err| err.to_string())?;
	let output = serde_json::to_string_pretty(&sidechain_params).map_err(|err| err.to_string())?;
	Ok(output)
}
