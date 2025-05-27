//! # Genesis UTXO Retrieval
//!
//! Query the genesis UTXO from Partner Chain on-chain storage.
//! This module provides runtime API access to retrieve the genesis UTXO
//! that identifies a specific Partner Chain instance.
//!
//! ## Process Overview
//!
//! 1. Connect to Partner Chain runtime API
//! 2. Query genesis UTXO from on-chain storage
//! 3. Format output as JSON with genesis UTXO information
//!
//! ## CLI Integration
//!
//! ```bash
//! partner-chains-cli get-genesis-utxo
//! ```
//!
//! ## Output Format
//!
//! ```json
//! {
//!   "genesis_utxo": "e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efa#4"
//! }
//! ```
//!
//! ## Integration Requirements
//!
//! This function requires:
//! - Active connection to Partner Chain node
//! - Runtime API access with `GetGenesisUtxo` capability
//! - Valid blockchain client with header backend support

use sidechain_domain::UtxoId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sp_sidechain::GetGenesisUtxo;
use std::sync::Arc;

/// Retrieve the genesis UTXO from on-chain storage.
///
/// Queries the Partner Chain runtime API to fetch the genesis UTXO that uniquely
/// identifies the Partner Chain instance. The genesis UTXO is stored in on-chain
/// storage and accessible through the runtime API.
///
/// ## Process
///
/// 1. Access runtime API from blockchain client
/// 2. Get current best block hash
/// 3. Query genesis UTXO from runtime storage
/// 4. Format result as JSON string
///
/// ## Type Parameters
///
/// - `B`: Block type implementing `BlockT`
/// - `C`: Client type providing runtime API and header backend access
///
/// ## Parameters
///
/// - `client`: Arc-wrapped blockchain client with runtime API access
///
/// ## Returns
///
/// `Result<String, String>` containing:
/// - `Ok(String)`: JSON-formatted genesis UTXO information
/// - `Err(String)`: Error message if query or serialization fails
///
/// ## Output Format
///
/// ```json
/// {
///   "genesis_utxo": "e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efa#4"
/// }
/// ```
///
/// ## Errors
///
/// Returns error string if:
/// - Runtime API call fails
/// - Genesis UTXO is not available
/// - JSON serialization fails
///
/// ## Example Usage
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use toolkit_cli_commands::get_genesis_utxo;
///
/// async fn query_genesis_utxo(client: Arc<impl ProvideRuntimeApi<Block> + HeaderBackend<Block>>) {
///     match get_genesis_utxo::execute(client).await {
///         Ok(json_output) => println!("{}", json_output),
///         Err(error) => eprintln!("Failed to query genesis UTXO: {}", error),
///     }
/// }
/// ```
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
/// Contains the genesis UTXO that identifies the Partner Chain instance.
/// This structure is serialized to JSON for CLI output.
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Debug)]
struct Output {
	/// Genesis UTXO identifying the Partner Chain instance
	pub genesis_utxo: UtxoId,
}
