use sidechain_domain::UtxoId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sp_sidechain::GetGenesisUtxo;
use std::sync::Arc;

/// Retrieves the genesis UTXO from the Partner Chain's on-chain storage.
///
/// This function queries the runtime storage to obtain the genesis UTXO that
/// uniquely identifies the Partner Chain instance. The genesis UTXO is a critical
/// identifier used throughout the Partner Chain system for binding operations
/// to a specific chain instance and preventing cross-chain replay attacks.
///
/// ## Runtime API Integration
///
/// The function uses the Substrate runtime API to query the genesis UTXO from
/// the chain's storage. It operates on the best (latest) block to ensure the
/// most current state is queried.
///
/// # Type Parameters
/// * `B` - Block type implementing the `BlockT` trait
/// * `C` - Client type providing runtime API access and header backend functionality
///
/// # Arguments
/// * `client` - Arc-wrapped client instance for blockchain interaction
///
/// # Returns
/// * `Ok(String)` - JSON-formatted output containing the genesis UTXO
/// * `Err(String)` - Error message describing the failure (runtime API error or JSON serialization error)
///
/// # Errors
/// This function will return an error if:
/// - The runtime API call fails (e.g., storage corruption, API version mismatch)
/// - JSON serialization of the output fails
///
/// # Output Format
/// The returned JSON contains a single field:
/// - `genesis_utxo`: The UTXO identifier in standard format (transaction_hash#output_index)
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
///
/// This struct represents the structured output format returned by the
/// genesis UTXO query operation. It provides a consistent JSON format
/// for external tools and scripts that need to obtain the Partner Chain's
/// genesis UTXO identifier.
///
/// ## Serialization
///
/// The struct implements both `Serialize` and `Deserialize` to support
/// bidirectional JSON conversion, enabling both output generation and
/// result parsing by consuming applications.
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Debug)]
struct Output {
	/// The genesis UTXO that uniquely identifies this Partner Chain instance
	pub genesis_utxo: UtxoId,
}
