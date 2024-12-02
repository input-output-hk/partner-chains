//! OgmiosClient implementation with jsonrpsee.
//! Major drawback is that it swallows the error response from the server in case of 400 Bad Request.

use crate::{OgmiosClient, OgmiosClientError, OgmiosParams};
use jsonrpsee::{
	core::{client::ClientT, traits::ToRpcParams, ClientError},
	http_client::HttpClient,
};
use serde::de::DeserializeOwned;

fn request_to_json(req: impl ToRpcParams) -> Result<String, OgmiosClientError> {
	let json_str = match req.to_rpc_params().expect("Parameters are correct") {
		None => "{}".to_string(),
		Some(req) => serde_json::to_string(&req).expect("Request params are valid json"),
	};
	Ok(json_str)
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
		log::debug!("request: {}", request_to_json(params.clone())?);
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
