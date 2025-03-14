//! Basic Sidechain Types returned by RPC endpoints

use jsonrpsee::types::{ErrorObject, ErrorObjectOwned};
use serde::{Deserialize, Serialize};
use sp_core::offchain::Timestamp;
use std::fmt::Debug;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetStatusResponse {
	pub sidechain: SidechainData,
	pub mainchain: MainchainData,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SidechainData {
	pub epoch: u64,
	pub slot: u64,
	pub next_epoch_timestamp: Timestamp,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MainchainData {
	pub epoch: u32,
	pub slot: u64,
	pub next_epoch_timestamp: Timestamp,
}

/// Errors that occur on the client RPC `sidechain_getStatus`
#[derive(Debug)]
pub enum GetStatusRpcError {
	CannotConvertSidechainSlotToTimestamp,
	CannotConvertTimestampToMainchainData,
}

impl From<GetStatusRpcError> for ErrorObjectOwned {
	fn from(e: GetStatusRpcError) -> Self {
		ErrorObject::owned::<u8>(-1, format!("{e:?}"), None)
	}
}
