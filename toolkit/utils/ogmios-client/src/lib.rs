//! This module provides a high-level API for interacting with the Ogmios JSON-RPC API.
//!
//! Ogmios is a JSON-RPC server that provides a high-level API for interacting with the Cardano blockchain.
//! It can be accessed via a HTTP or WebSocket connection.
//!
//! More information about Ogmios API can be found at https://ogmios.dev/api/

#[cfg(feature = "jsonrpsee-client")]
pub mod jsonrpsee;
pub mod query_ledger_state;
pub mod query_network;
pub mod transactions;
pub mod types;

use ::jsonrpsee::core::{
	params::{ArrayParams, ObjectParams},
	traits::ToRpcParams,
};
use serde::de::DeserializeOwned;
use std::collections::HashMap;

#[derive(Debug, Clone, thiserror::Error)]
/// Represents an error that can occur when interacting with the Ogmios JSON-RPC API.
pub enum OgmiosClientError {
	#[error("Couldn't construct parameters: '{0}'")]
	/// Represents an error that can occur when incorrect parameters are provided to the Ogmios JSON-RPC API.
	ParametersError(String),
	#[error("JsonRPC request failed: '{0}'")]
	/// Represents an error that can occur when the JSON-RPC request fails.
	RequestError(String),
	#[error("Could not parse response: '{0}'")]
	/// Represents an error that can occur when the response from the Ogmios JSON-RPC API cannot be parsed.
	ResponseError(String),
}

/// Trait for interacting with the Ogmios JSON-RPC API.
pub trait OgmiosClient {
	#[allow(async_fn_in_trait)]
	/// Sends a JSON-RPC request to the Ogmios server and returns the response.
	async fn request<T: DeserializeOwned>(
		&self,
		method: &str,
		params: OgmiosParams,
	) -> Result<T, OgmiosClientError>;
}

#[derive(Clone)]
/// Enum representing the parameters for a JSON-RPC request to the Ogmios server.
pub enum OgmiosParams {
	/// Represents positional parameters.
	Positional(Vec<serde_json::Value>),
	/// Represents named parameters.
	ByName(HashMap<&'static str, serde_json::Value>),
}

impl OgmiosParams {
	pub fn empty_positional() -> Self {
		OgmiosParams::Positional(Vec::new())
	}

	pub fn empty_by_name() -> Self {
		OgmiosParams::ByName(HashMap::new())
	}
}

impl ToRpcParams for OgmiosParams {
	fn to_rpc_params(self) -> Result<Option<Box<serde_json::value::RawValue>>, serde_json::Error> {
		match self {
			OgmiosParams::ByName(map) => {
				let mut object_params = ObjectParams::new();
				map.into_iter().try_for_each(|(k, v)| object_params.insert(k, v))?;
				object_params.to_rpc_params()
			},
			OgmiosParams::Positional(v) => {
				let mut array_params = ArrayParams::new();
				v.into_iter().try_for_each(|v| array_params.insert(v))?;
				array_params.to_rpc_params()
			},
		}
	}
}

/// Builder for named parameters.
pub struct ByNameParamsBuilder {
	/// The parameters.
	params: HashMap<&'static str, serde_json::Value>,
}

impl Default for ByNameParamsBuilder {
	fn default() -> Self {
		Self::new()
	}
}

impl ByNameParamsBuilder {
	/// Creates a new builder for named parameters.
	pub fn new() -> Self {
		ByNameParamsBuilder { params: HashMap::new() }
	}

	/// Inserts a new parameter into the builder.
	pub fn insert<T: serde::Serialize>(
		self,
		key: &'static str,
		value: T,
	) -> Result<Self, OgmiosClientError> {
		let value = serde_json::to_value(value)
			.map_err(|e| OgmiosClientError::ParametersError(e.to_string()))?;
		let mut params = self.params;
		params.insert(key, value);
		Ok(Self { params })
	}

	/// Builds the named parameters.
	pub fn build(self) -> OgmiosParams {
		OgmiosParams::ByName(self.params)
	}
}
