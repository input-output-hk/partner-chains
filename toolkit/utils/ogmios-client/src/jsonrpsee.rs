//! OgmiosClient implementation with jsonrpsee.
//! Major drawback is that it swallows the error response from the server in case of 400 Bad Request.

use std::io::Write;

use crate::{OgmiosClient, OgmiosClientError, OgmiosParams};
use jsonrpsee::{
	core::client::ClientT,
	core::params::{ArrayParams, ObjectParams},
	http_client::HttpClient,
};
use serde::de::DeserializeOwned;

const CONTRACTLOG_FILE: &str = "contractlog.json";

macro_rules! contractlog {
	($($arg:tt)*) => {
		add_to_contractlog(&format!($($arg)*))
	};
}

pub fn add_to_contractlog(msg: &str) {
	let mut handle = match std::fs::OpenOptions::new()
		.write(true)
		.append(true)
		.create(true)
		.open(CONTRACTLOG_FILE)
	{
		Ok(handle) => handle,
		Err(err) => {
			eprintln!("Failed to open contractlog file {CONTRACTLOG_FILE}: {err}");
			return;
		},
	};

	let Ok(_) = handle.write_all(msg.as_bytes()) else {
		eprintln!("Failed to write contractlog to file {CONTRACTLOG_FILE}");
		return;
	};
}

impl OgmiosClient for HttpClient {
	async fn request<T: DeserializeOwned + std::fmt::Debug>(
		&self,
		method: &str,
		params: OgmiosParams,
	) -> Result<T, OgmiosClientError> {
		let response = match params {
			OgmiosParams::ByName(map) => {
				let mut object_params = ObjectParams::new();
				map.into_iter()
					.try_for_each(|(k, v)| object_params.insert(k, v))
					.map_err(serde_error_to_parameters_error)?;
				contractlog!("request: {object_params:?}");
				Ok(ClientT::request(self, method, object_params).await?)
			},
			OgmiosParams::Positional(v) => {
				let mut array_params = ArrayParams::new();
				v.into_iter()
					.try_for_each(|v| array_params.insert(v))
					.map_err(serde_error_to_parameters_error)?;
				contractlog!("request: {array_params:?}");
				Ok(ClientT::request(self, method, array_params).await?)
			},
		};

		contractlog!("response: {response:?}");

		response
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

fn serde_error_to_parameters_error(e: serde_json::Error) -> OgmiosClientError {
	OgmiosClientError::ParametersError(e.to_string())
}
