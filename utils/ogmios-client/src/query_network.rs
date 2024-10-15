//! Queries that start with `queryNetwork/`.

use crate::{types::SlotLength, OgmiosClient, OgmiosClientError, OgmiosParams};
use fraction::Decimal;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

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
	pub network_magic: u32,
	pub security_parameter: u32,
	#[serde(deserialize_with = "parse_fraction")]
	pub active_slots_coefficient: Decimal,
	pub epoch_length: u32,
	pub slot_length: SlotLength,
	#[serde(deserialize_with = "time::serde::iso8601::deserialize")]
	pub start_time: time::OffsetDateTime,
}

fn parse_fraction<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
	D: Deserializer<'de>,
{
	let buf = String::deserialize(deserializer)?;
	Decimal::from_str(&buf).map_err(serde::de::Error::custom)
}
