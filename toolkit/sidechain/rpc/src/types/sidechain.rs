//! Response types returned by RPC endpoints for Sidechain pallet

use jsonrpsee::types::{ErrorObject, ErrorObjectOwned};
use serde::{Deserialize, Serialize};
use sp_core::offchain::Timestamp;
use std::fmt::Debug;

/// Response type of [SidechainRpcApiServer::get_status]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetStatusResponse {
	/// Partner Chain epoch and slot information
	pub sidechain: SidechainData,
	/// Cardano main chain epoch and slot information
	pub mainchain: MainchainData,
}

/// Data about current Partner Chain epoch and slot
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SidechainData {
	/// Current Partner Chain epoch number
	pub epoch: u64,
	/// Current Partner Chain slot number
	pub slot: u64,
	/// Timestamp of the next Partner Chain epoch start
	pub next_epoch_timestamp: Timestamp,
}

/// Data about current Cardano main chain epoch and slot
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MainchainData {
	/// Current Cardano main chain epoch number
	pub epoch: u32,
	/// Current Cardano main chain slot number
	pub slot: u64,
	/// Timestamp of the next Cardano main chain epoch start
	pub next_epoch_timestamp: Timestamp,
}

/// Error type returned by [SidechainRpcApiServer::get_status]
#[derive(Debug)]
pub enum GetStatusRpcError {
	/// Signals that the server could not convert Partner Chain slot number to timestamp
	CannotConvertSidechainSlotToTimestamp,
	/// Signals that the server could not convert a timestamp to Cardano main chain
	CannotConvertTimestampToMainchainData,
}

impl From<GetStatusRpcError> for ErrorObjectOwned {
	fn from(e: GetStatusRpcError) -> Self {
		ErrorObject::owned::<u8>(-1, format!("{e:?}"), None)
	}
}
