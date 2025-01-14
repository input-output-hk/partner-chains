#![cfg(feature = "jsonrpsee-client")]

use fraction::{Decimal, Fraction};
use jsonrpsee::types::{ErrorCode, ErrorObject};
use ogmios_client::{
	jsonrpsee::client_for_url,
	query_network::{QueryNetwork, ShelleyGenesisConfigurationResponse},
	types::SlotLength,
};
use serde_json::{json, Value};
use sidechain_domain::NetworkType;
use time::OffsetDateTime;

mod server;

#[tokio::test]
async fn shelley_genesis_configuration() {
	let address = server::for_single_test("queryNetwork/genesisConfiguration", |params| {
		let _ = params
			.parse()
			.ok()
			.and_then(|params: Value| params.pointer("/era").cloned())
			.filter(|era| *era == serde_json::Value::String("shelley".to_string()))
			.ok_or(ErrorObject::owned(
				ErrorCode::InvalidParams.code(),
				"invalid era parameter",
				Some(r#"Expected "params": {"era": "shelley"}}"#),
			))?;

		// This is real answer from Ogmios with the most of fields removed for brevity
		Ok(json!({
		  "era": "shelley",
		  "startTime": "2022-10-25T00:00:00Z",
		  "networkMagic": 2,
		  "network": "testnet",
		  "activeSlotsCoefficient": "1/20",
		  "securityParameter": 432,
		  "epochLength": 86400,
		  "slotLength": {
			"milliseconds": 1000
		  },
		}))
	})
	.await
	.unwrap();
	let client = client_for_url(&format!("http://{address}")).await.unwrap();
	let genesis_configuration = client.shelley_genesis_configuration().await.unwrap();
	assert_eq!(
		genesis_configuration,
		ShelleyGenesisConfigurationResponse {
			network_magic: 2,
			network: NetworkType::Testnet,
			start_time: OffsetDateTime::from_unix_timestamp(1666656000).unwrap(),
			security_parameter: 432,
			epoch_length: 86400,
			active_slots_coefficient: Decimal::from_fraction(Fraction::new(1u64, 20u64)),
			slot_length: SlotLength { milliseconds: 1000 },
		}
	)
}
