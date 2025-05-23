//! OgmiosClient implementation with jsonrpsee.
//! Major drawback is that it swallows the error response from the server in case of 400 Bad Request.

use crate::{OgmiosClient, OgmiosClientError, OgmiosParams, query_ledger_state::QueryLedgerState};
use jsonrpsee::{
	core::{ClientError, client::ClientT, traits::ToRpcParams},
	http_client::{HttpClient, HttpClientBuilder},
	ws_client::{WsClient, WsClientBuilder},
};
use serde::de::DeserializeOwned;
use serde_json::json;

/// Converts the method and parameters to a JSON-RPC request string.
fn request_to_json(method: &str, params: impl ToRpcParams) -> Result<String, OgmiosClientError> {
	let params = params
		.to_rpc_params()
		.map_err(|err| OgmiosClientError::ParametersError(err.to_string()))?
		.unwrap_or_default();

	let req = json!({
		"method": method,
		"params": params
	});

	serde_json::to_string(&req).map_err(|err| OgmiosClientError::ParametersError(err.to_string()))
}

/// Converts the response to a JSON string.
fn response_to_json(resp: &Result<serde_json::Value, ClientError>) -> String {
	match &resp {
		Ok(resp) => serde_json::to_string(&resp).unwrap(),
		Err(jsonrpsee::core::ClientError::Call(err)) => serde_json::to_string(&err).unwrap(),
		Err(err) => err.to_string(),
	}
}

/// Enum that represents the ogmios client that works either with HTTP or WebSockets.
pub enum OgmiosClients {
	HttpClient(HttpClient),
	WsClient(WsClient),
}

/// Returns client that works either with HTTP or WebSockets.
/// HTTP does not return JSON-RPC error body in case of 400 Bad Request.
pub async fn client_for_url(addr: &str) -> Result<OgmiosClients, String> {
	if addr.starts_with("http") || addr.starts_with("https") {
		let client = HttpClientBuilder::default()
			.build(addr)
			.map_err(|e| format!("Couldn't create HTTP client: {}", e))?;

		let http_client = OgmiosClients::HttpClient(client);

		// We make a call to get_tip to test HTTP connection
		http_client
			.get_tip()
			.await
			.map_err(|e| format!("Failed to test HTTP connection: {}", e))?;

		Ok(http_client)
	} else if addr.starts_with("ws") || addr.starts_with("wss") {
		let client = WsClientBuilder::default()
			.build(addr.to_owned())
			.await
			.map_err(|e| format!("Couldn't create WebSockets client: {}", e))?;
		Ok(OgmiosClients::WsClient(client))
	} else {
		Err(format!("Invalid Schema of URL: '{}'. Expected http, https, ws or wss.", addr))
	}
}

impl OgmiosClient for OgmiosClients {
	/// Sends a JSON-RPC request to the Ogmios server and returns the response.
	async fn request<T: DeserializeOwned>(
		&self,
		method: &str,
		params: OgmiosParams,
	) -> Result<T, OgmiosClientError> {
		log::debug!("request: {}", request_to_json(method, params.clone())?);
		let response = match self {
			OgmiosClients::HttpClient(client) => {
				ClientT::request::<serde_json::Value, _>(client, method, params).await
			},
			OgmiosClients::WsClient(client) => {
				ClientT::request::<serde_json::Value, _>(client, method, params).await
			},
		};
		log::debug!("response: {}", response_to_json(&response));

		serde_json::from_value(response?)
			.map_err(|err| OgmiosClientError::ResponseError(err.to_string()))
	}
}

impl From<jsonrpsee::core::ClientError> for OgmiosClientError {
	fn from(e: jsonrpsee::core::ClientError) -> Self {
		match e {
			jsonrpsee::core::ClientError::ParseError(e) => {
				OgmiosClientError::ResponseError(e.to_string())
			},
			e => OgmiosClientError::RequestError(e.to_string()),
		}
	}
}
