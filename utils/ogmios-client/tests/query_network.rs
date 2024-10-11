use jsonrpsee::{
	http_client::HttpClient,
	server::Server,
	types::{ErrorCode, ErrorObject, ErrorObjectOwned, Params},
	Extensions, RpcModule,
};
use ogmios_client::{
	query_network::{QueryNetwork, ShelleyGenesisConfigurationResponse},
	types::SlotLength,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use time::OffsetDateTime;
use tokio::sync::OnceCell;

const SERVER_ADDRESS: OnceCell<SocketAddr> = OnceCell::const_new();

async fn get_server_address() -> SocketAddr {
	SERVER_ADDRESS
		.get_or_init(|| async { run_server().await.unwrap() })
		.await
		.clone()
}

#[tokio::test]
async fn shelley_genesis_configuration() {
	let address = get_server_address().await;
	let client = HttpClient::builder().build(format!("http://{address}")).unwrap();
	let genesis_configuration = client.shelley_genesis_configuration().await.unwrap();
	assert_eq!(
		genesis_configuration,
		ShelleyGenesisConfigurationResponse {
			network_magic: 2,
			start_time: OffsetDateTime::from_unix_timestamp(1666656000).unwrap(),
			security_parameter: 432,
			epoch_length: 86400,
			active_slots_coefficient: "1/20".to_string(),
			slot_length: SlotLength { milliseconds: 1000 },
		}
	)
}

async fn run_server() -> anyhow::Result<SocketAddr> {
	let server = Server::builder().build("127.0.0.1:0".parse::<SocketAddr>()?).await?;
	let mut module = RpcModule::new(());
	module.register_method("queryNetwork/genesisConfiguration", test_handler)?;
	let addr = server.local_addr()?;
	let handle = server.start(module);
	// It will stop when test main exists.
	tokio::spawn(handle.stopped());
	Ok(addr)
}

fn test_handler(params: Params, _ctx: &(), _e: &Extensions) -> Result<Value, ErrorObjectOwned> {
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
}
