//! Queries that start with `queryNetwork/`.

use crate::{OgmiosClient, OgmiosClientError, OgmiosParams, types::SlotLength};
use fraction::Decimal;
use serde::Deserialize;
use serde_json::Value;
use sidechain_domain::NetworkType;
use std::collections::HashMap;

/// Trait that defines the methods for querying the network.
pub trait QueryNetwork {
	#[allow(async_fn_in_trait)]
	/// Returns the Shelley genesis configuration.
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
/// Represents the Shelley genesis configuration.
pub struct ShelleyGenesisConfigurationResponse {
	/// The network magic number.
	pub network_magic: u32,
	/// The network type.
	pub network: NetworkType,
	/// The security parameter.
	pub security_parameter: u32,
	/// The active slots coefficient.
	#[serde(deserialize_with = "crate::types::parse_fraction_decimal")]
	pub active_slots_coefficient: Decimal,
	/// The epoch length.
	pub epoch_length: u32,
	/// The slot length.
	pub slot_length: SlotLength,
	/// The start time.
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
