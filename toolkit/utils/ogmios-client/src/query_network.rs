//! Queries that start with `queryNetwork/`.

use crate::{OgmiosClient, OgmiosClientError, OgmiosParams, types::SlotLength};
use fraction::Decimal;
use serde::Deserialize;
use serde_json::Value;
use sidechain_domain::NetworkType;
use std::collections::HashMap;

pub trait QueryNetwork {
	#[allow(async_fn_in_trait)]
	async fn shelley_genesis_configuration(
		&self,
	) -> Result<ShelleyGenesisConfigurationResponse, OgmiosClientError>;
}

impl<T: OgmiosClient> QueryNetwork for T {
	async fn shelley_genesis_configuration(
		&self,
	) -> Result<ShelleyGenesisConfigurationResponse, OgmiosClientError> {
		let mut params = HashMap::new();
		params.insert("era", Value::String("shelley".to_string()));
		self.request("queryNetwork/genesisConfiguration", OgmiosParams::ByName(params))
			.await
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShelleyGenesisConfigurationResponse {
	pub network_magic: u32,
	pub network: NetworkType,
	pub security_parameter: u32,
	#[serde(deserialize_with = "crate::types::parse_fraction_decimal")]
	pub active_slots_coefficient: Decimal,
	pub epoch_length: u32,
	pub slot_length: SlotLength,
	#[serde(deserialize_with = "time::serde::iso8601::deserialize")]
	pub start_time: time::OffsetDateTime,
}

impl Default for ShelleyGenesisConfigurationResponse {
	fn default() -> Self {
		Self {
			network_magic: Default::default(),
			network: Default::default(),
			security_parameter: Default::default(),
			active_slots_coefficient: Default::default(),
			epoch_length: Default::default(),
			slot_length: Default::default(),
			start_time: time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
		}
	}
}
