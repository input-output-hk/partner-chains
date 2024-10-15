//! Queries that start with `queryLedgerState/`.

use crate::{
	types::{SlotLength, TimeSeconds},
	OgmiosClient, OgmiosClientError, OgmiosParams,
};
use serde::Deserialize;

pub trait QueryLedgerState: OgmiosClient {
	#[allow(async_fn_in_trait)]
	async fn era_summaries(&self) -> Result<Vec<EraSummary>, OgmiosClientError> {
		self.request("queryLedgerState/eraSummaries", OgmiosParams::empty_positional())
			.await
	}
}

impl<T: OgmiosClient> QueryLedgerState for T {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EraSummary {
	pub start: EpochBoundary,
	pub end: EpochBoundary,
	pub parameters: EpochParameters,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct EpochBoundary {
	pub time: TimeSeconds,
	pub slot: u64,
	pub epoch: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EpochParameters {
	pub epoch_length: u32,
	pub slot_length: SlotLength,
	pub safe_zone: u32,
}
