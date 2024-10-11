//! This module provides a high-level API for interacting with the Ogmios JSON-RPC API.

#[cfg(feature = "jsonrpsee-client")]
pub mod jsonrpsee;
pub mod query_ledger_state;
pub mod query_network;
pub mod types;

use serde::de::DeserializeOwned;
use std::collections::HashMap;

#[derive(Debug, Clone, thiserror::Error)]
pub enum OgmiosClientError {
	#[error("Couldn't construct parameters: '{0}'")]
	ParametersError(String),
	#[error("JsonRPC request failed: '{0}'")]
	RequestError(String),
	#[error("Could not parse response: '{0}'")]
	ResponseError(String),
}

pub trait OgmiosClient {
	#[allow(async_fn_in_trait)]
	async fn request<T: DeserializeOwned>(
		&self,
		method: &str,
		params: OgmiosParams,
	) -> Result<T, OgmiosClientError>;
}

pub enum OgmiosParams {
	Positional(Vec<serde_json::Value>),
	ByName(HashMap<&'static str, serde_json::Value>),
}

impl OgmiosParams {
	pub fn empty_positional() -> Self {
		OgmiosParams::Positional(Vec::new())
	}
}
