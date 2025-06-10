use sidechain_domain::UtxoId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sp_sidechain::GetGenesisUtxo;
use std::sync::Arc;

/// Queries the genesis UTXO from Partner Chain storage via runtime API.
pub async fn execute<B, C>(client: Arc<C>) -> Result<String, String>
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

/// Output structure for genesis UTXO query results.
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Debug)]
struct Output {
	/// The genesis UTXO that uniquely identifies this Partner Chain instance
	pub genesis_utxo: UtxoId,
}
