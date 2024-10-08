//! Queries that start with `queryNetwork/`.

use crate::{types::SlotLength, OgmiosClient, OgmiosClientError, OgmiosParams};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

pub trait QueryNetwork: OgmiosClient {
	#[allow(async_fn_in_trait)]
	async fn shelley_genesis_configuration(
		&self,
	) -> Result<ShelleyGenesisConfigurationResponse, OgmiosClientError> {
		let mut params = HashMap::new();
		params.insert("era", Value::String("shelley".to_string()));
		self.request("queryNetwork/genesisConfiguration", OgmiosParams::ByName(params))
			.await
	}
}

impl<T: OgmiosClient> QueryNetwork for T {}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShelleyGenesisConfigurationResponse {
	pub security_parameter: u32,
	// Ogmios returns active_slots_coefficient as string representing rational number, like "1/20"
	pub active_slots_coefficient: String,
	pub epoch_length: u32,
	pub slot_length: SlotLength,
	#[serde(deserialize_with = "time::serde::iso8601::deserialize")]
	pub start_time: time::OffsetDateTime,
}
