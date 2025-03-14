use crate::GetGenesisUtxo;
use sidechain_domain::UtxoId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

pub async fn get_genesis_utxo<B, C>(client: Arc<C>) -> Result<String, String>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: GetGenesisUtxo<B>,
	C: HeaderBackend<B>,
{
	let api = client.runtime_api();
	let best_block = client.info().best_hash;
	let genesis_utxo = api.genesis_utxo(best_block).map_err(|err| err.to_string())?;
	let output =
		serde_json::to_string_pretty(&Output { genesis_utxo }).map_err(|err| err.to_string())?;
	Ok(output)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Debug)]
pub struct Output {
	pub genesis_utxo: UtxoId,
}
