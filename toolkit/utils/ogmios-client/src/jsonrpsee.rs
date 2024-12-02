//! OgmiosClient implementation with jsonrpsee.
//! Major drawback is that it swallows the error response from the server in case of 400 Bad Request.

use std::io::Write;

use crate::{OgmiosClient, OgmiosClientError, OgmiosParams};
use jsonrpsee::{
	core::{
		client::ClientT,
		params::{ArrayParams, ObjectParams},
		traits::ToRpcParams,
		ClientError,
	},
	http_client::HttpClient,
};
use serde::de::DeserializeOwned;

const CONTRACTLOG_FILE: &str = "contractlog.json";

macro_rules! contractlog {
	($($arg:tt)*) => {
		add_to_contractlog(&format!("{}\n", format!($($arg)*)))
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
		let response = match params {
			OgmiosParams::ByName(map) => {
				let mut object_params = ObjectParams::new();
				map.into_iter()
					.try_for_each(|(k, v)| object_params.insert(k, v))
					.map_err(serde_error_to_parameters_error)?;
				contractlog!("request: {}", request_to_json(object_params.clone())?);
				ClientT::request::<serde_json::Value, _>(self, method, object_params).await
			},
			OgmiosParams::Positional(v) => {
				let mut array_params = ArrayParams::new();
				v.into_iter()
					.try_for_each(|v| array_params.insert(v))
					.map_err(serde_error_to_parameters_error)?;
				contractlog!("request: {}", request_to_json(array_params.clone())?);
				ClientT::request::<serde_json::Value, _>(self, method, array_params).await
			},
		};

		contractlog!("response: {}", response_to_json(&response));

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

fn serde_error_to_parameters_error(e: serde_json::Error) -> OgmiosClientError {
	OgmiosClientError::ParametersError(e.to_string())
}
