//! OgmiosClient implementation with jsonrpsee.
//! Major drawback is that it swallows the error response from the server in case of 400 Bad Request.

use crate::{OgmiosClient, OgmiosClientError, OgmiosParams};
use jsonrpsee::{
	core::{client::ClientT, traits::ToRpcParams, ClientError},
	http_client::HttpClient,
};
use serde::de::DeserializeOwned;
use serde_json::json;

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

fn response_to_json(resp: &Result<serde_json::Value, ClientError>) -> String {
	match &resp {
		Ok(resp) => serde_json::to_string(&resp).unwrap(),
		Err(jsonrpsee::core::ClientError::Call(err)) => serde_json::to_string(&err).unwrap(),
		Err(err) => err.to_string(),
	}
}

impl OgmiosClient for HttpClient {
	async fn request<T: DeserializeOwned>(
		&self,
		method: &str,
		params: OgmiosParams,
	) -> Result<T, OgmiosClientError> {
		log::debug!("request: {}", request_to_json(method, params.clone())?);
		let response = ClientT::request::<serde_json::Value, _>(self, method, params).await;

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
