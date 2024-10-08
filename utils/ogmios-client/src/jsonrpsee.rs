//! OgmiosClient implementation with jsonrpsee.
//! Major drawback is that it swallows the error response from the server in case of 400 Bad Request.

use crate::{OgmiosClient, OgmiosClientError, OgmiosParams};
use jsonrpsee::core::params::ArrayParams;
use jsonrpsee::{
	core::{client::ClientT, params::ObjectParams},
	http_client::HttpClient,
};
use serde::de::DeserializeOwned;

impl OgmiosClient for HttpClient {
	async fn request<T: DeserializeOwned>(
		&self,
		method: &str,
		params: OgmiosParams,
	) -> Result<T, OgmiosClientError> {
		match params {
			OgmiosParams::ByName(map) => {
				let mut object_params = ObjectParams::new();
				map.into_iter()
					.try_for_each(|(k, v)| object_params.insert(k, v))
					.map_err(serde_error_to_ogmios_error)?;
				ClientT::request(self, method, object_params)
					.await
					.map_err(jsonrpsee_to_ogmios_error)
			},
			OgmiosParams::Positional(v) => {
				let mut array_params = ArrayParams::new();
				v.into_iter()
					.try_for_each(|v| array_params.insert(v))
					.map_err(serde_error_to_ogmios_error)?;
				ClientT::request(self, method, array_params)
					.await
					.map_err(jsonrpsee_to_ogmios_error)
			},
		}
	}
}

fn jsonrpsee_to_ogmios_error(e: jsonrpsee::core::ClientError) -> OgmiosClientError {
	match e {
		jsonrpsee::core::ClientError::ParseError(e) => {
			OgmiosClientError::ResponseError(e.to_string())
		},
		e => OgmiosClientError::RequestError(e.to_string()),
	}
}

fn serde_error_to_ogmios_error(e: serde_json::Error) -> OgmiosClientError {
	OgmiosClientError::ParametersError(e.to_string())
}
