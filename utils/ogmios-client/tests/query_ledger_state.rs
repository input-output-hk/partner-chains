#![cfg(feature = "jsonrpsee-client")]

use jsonrpsee::http_client::HttpClient;
use ogmios_client::{
	query_ledger_state::{EpochBoundary, EpochParameters, EraSummary, QueryLedgerState},
	types::{SlotLength, TimeSeconds},
};
use serde_json::json;

mod server;

#[tokio::test]
async fn era_summaries() {
	let address = server::for_single_test("queryLedgerState/eraSummaries", |_| {
		Ok(json!([{
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
		]))
	})
	.await
	.unwrap();
	let client = HttpClient::builder().build(format!("http://{address}")).unwrap();
	let era_summaries = client.era_summaries().await.unwrap();
	assert_eq!(era_summaries.len(), 3);
	assert_eq!(
		era_summaries[0],
		EraSummary {
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
