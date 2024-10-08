use jsonrpsee::{http_client::HttpClient, server::Server, RpcModule};
use ogmios_client::{
	query_ledger_state::{EpochBoundary, EpochParameters, EraSummariesResponse, QueryLedgerState},
	types::{SlotLength, TimeSeconds},
};
use serde_json::json;
use std::net::SocketAddr;
use tokio::sync::OnceCell;

const SERVER_ADDRESS: OnceCell<SocketAddr> = OnceCell::const_new();

async fn get_server_address() -> SocketAddr {
	SERVER_ADDRESS
		.get_or_init(|| async { run_server().await.unwrap() })
		.await
		.clone()
}

#[tokio::test]
async fn era_summaries() {
	let address = get_server_address().await;
	let client = HttpClient::builder().build(format!("http://{address}")).unwrap();
	let era_summaries = client.era_summaries().await.unwrap();
	assert_eq!(era_summaries.len(), 3);
	assert_eq!(
		era_summaries[0],
		EraSummariesResponse {
			start: EpochBoundary { time: TimeSeconds { seconds: 0 }, slot: 0, epoch: 0 },
			end: EpochBoundary { time: TimeSeconds { seconds: 1728000 }, slot: 86400, epoch: 4 },
			parameters: EpochParameters {
				epoch_length: 21600,
				slot_length: SlotLength { milliseconds: 20000 },
				safe_zone: 4320
			}
		}
	)
}

async fn run_server() -> anyhow::Result<SocketAddr> {
	let server = Server::builder().build("127.0.0.1:0".parse::<SocketAddr>()?).await?;
	let mut module = RpcModule::new(());
	module.register_method("queryLedgerState/eraSummaries", |_, _, _| {
		// This is real answer from Ogmios, trimmed down to 3 eras for brevity
		json!([
		  {
			"start": {
			  "time": {
				"seconds": 0
			  },
			  "slot": 0,
			  "epoch": 0
			},
			"end": {
			  "time": {
				"seconds": 1728000
			  },
			  "slot": 86400,
			  "epoch": 4
			},
			"parameters": {
			  "epochLength": 21600,
			  "slotLength": {
				"milliseconds": 20000
			  },
			  "safeZone": 4320
			}
		  },
		  {
			"start": {
			  "time": {
				"seconds": 1728000
			  },
			  "slot": 86400,
			  "epoch": 4
			},
			"end": {
			  "time": {
				"seconds": 2160000
			  },
			  "slot": 518400,
			  "epoch": 5
			},
			"parameters": {
			  "epochLength": 432000,
			  "slotLength": {
				"milliseconds": 1000
			  },
			  "safeZone": 129600
			}
		  },
		  {
			"start": {
			  "time": {
				"seconds": 70416000
			  },
			  "slot": 68774400,
			  "epoch": 163
			},
			"end": {
			  "time": {
				"seconds": 74736000
			  },
			  "slot": 73094400,
			  "epoch": 173
			},
			"parameters": {
			  "epochLength": 432000,
			  "slotLength": {
				"milliseconds": 1000
			  },
			  "safeZone": 129600
			}
		  }
		])
	})?;
	let addr = server.local_addr()?;
	let handle = server.start(module);
	// It will stop when test main exists.
	tokio::spawn(handle.stopped());
	Ok(addr)
}
