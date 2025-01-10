#![cfg(feature = "jsonrpsee-client")]

use hex_literal::hex;
use jsonrpsee::http_client::HttpClient;
use ogmios_client::{
	query_ledger_state::{
		EpochBoundary, EpochParameters, EraSummary, PlutusCostModels, ProtocolParametersResponse,
		QueryLedgerState, QueryUtxoByUtxoId, ReferenceScriptsCosts, ScriptExecutionPrices,
	},
	types::{Asset, OgmiosBytesSize, OgmiosTx, OgmiosUtxo, OgmiosValue, SlotLength, TimeSeconds},
};
use serde_json::json;
use sidechain_domain::UtxoId;

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

#[tokio::test]
async fn protocol_parameters() {
	let address = server::for_single_test("queryLedgerState/protocolParameters", |_| {
		Ok(json!({
		  "minFeeCoefficient": 44,
		  "minFeeConstant": {
			"ada": {
			  "lovelace": 155381
			}
		  },
		  "maxValueSize": {
			"bytes": 5000
		  },
		  "maxBlockBodySize": {
			"bytes": 90112
		  },
		  "maxTransactionSize": {
			"bytes": 16384
		  },
		  "stakeCredentialDeposit": {
			"ada": {
			  "lovelace": 2000000
			}
		  },
		  "stakePoolDeposit": {
			"ada": {
			  "lovelace": 500000000
			}
		  },
		  "collateralPercentage": 150,
		  "maxCollateralInputs": 3,
		  "minUtxoDepositCoefficient": 4310,
		  "plutusCostModels": {
			"plutus:v1": [
			  898148,
			  53384111,
			  14333,
			],
			"plutus:v2": [
			  43053543,
			  10
			],
			"plutus:v3": [
			  -900,
			  166917843,
			]
		  },
		  "scriptExecutionPrices": {
			"memory": "577/10000",
			"cpu": "721/10000000"
		  },
		  "minFeeReferenceScripts": {
			"base": 10.0,
			"range": 0,
			"multiplier": 2.0
		  }
		}))
	})
	.await
	.unwrap();
	let client = HttpClient::builder().build(format!("http://{address}")).unwrap();
	let parameters = client.query_protocol_parameters().await.unwrap();

	assert_eq!(
		parameters,
		ProtocolParametersResponse {
			min_fee_coefficient: 44,
			min_fee_constant: OgmiosValue::new_lovelace(155381),
			stake_pool_deposit: OgmiosValue::new_lovelace(500000000),
			stake_credential_deposit: OgmiosValue::new_lovelace(2000000),
			max_value_size: OgmiosBytesSize { bytes: 5000 },
			max_transaction_size: OgmiosBytesSize { bytes: 16384 },
			min_utxo_deposit_coefficient: 4310,
			script_execution_prices: ScriptExecutionPrices {
				memory: fraction::Ratio::new_raw(577, 10000),
				cpu: fraction::Ratio::new_raw(721, 10000000),
			},
			plutus_cost_models: PlutusCostModels {
				plutus_v1: vec![898148, 53384111, 14333],
				plutus_v2: vec![43053543, 10],
				plutus_v3: vec![-900, 166917843],
			},
			max_collateral_inputs: 3,
			collateral_percentage: 150,
			min_fee_reference_scripts: ReferenceScriptsCosts { base: 10.0 }
		}
	);
}

#[tokio::test]
async fn query_utxos() {
	let address = server::for_single_test("queryLedgerState/utxo", |_| {
		Ok(json!([
		  {
			"transaction": {
			  "id": "106b0d7d1544c97941777041699412fb7c8b94855210987327199620c0599580"
			},
			"index": 1,
			"address": "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
			"value": {
			  "ada": {
				"lovelace": 1356118
			  },
			  "e0d4479b3dbb53b1aecd48f7ef524a9cf166585923d91d9c72ed02cb": {
					  "707070": 18446744073709551615i128
			  }
			},
			"datum": "d8799fff"
		  },
		  {
			"transaction": {
			  "id": "c3f5e96605027d06b0836be6fc833b8340405c3caa7508282334182a2f650cf3"
			},
			"index": 7,
			"address": "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
			"value": {
			  "ada": {
				"lovelace": 2198596
			  }
			}
		  },
		]))
	})
	.await
	.unwrap();
	let client = HttpClient::builder().build(format!("http://{address}")).unwrap();
	let utxos = client
		.query_utxos(&[
			"addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy".into(),
			"addr_test1wq7vcwawqa29a5a2z7q8qs6k0cuvp6z2puvd8xx7vasuajq86paxz".into(),
		])
		.await
		.unwrap();
	assert_eq!(utxos.len(), 2);
	assert_eq!(
		utxos.first().unwrap().clone(),
		OgmiosUtxo {
			transaction: OgmiosTx {
				id: hex!("106b0d7d1544c97941777041699412fb7c8b94855210987327199620c0599580")
					.try_into()
					.unwrap()
			},
			index: 1,
			address: "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy".into(),
			value: OgmiosValue {
				lovelace: 1356118,
				native_tokens: {
					let mut map = std::collections::HashMap::new();
					map.insert(
						hex!("e0d4479b3dbb53b1aecd48f7ef524a9cf166585923d91d9c72ed02cb"),
						vec![Asset {
							name: hex!("707070").to_vec(),
							amount: 18446744073709551615i128,
						}],
					);
					map
				},
			},
			datum: Some(hex!("d8799fff").to_vec().into()),
			datum_hash: None,
			script: None
		}
	)
}

#[tokio::test]
async fn query_utxos_by_tx_hash() {
	let address = server::for_single_test("queryLedgerState/utxo", |_| {
		Ok(json!([
		  {
			"transaction": {
			  "id": "106b0d7d1544c97941777041699412fb7c8b94855210987327199620c0599580"
			},
			"index": 1,
			"address": "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
			"value": {
			  "ada": {
				"lovelace": 1356118
			  },
			  "e0d4479b3dbb53b1aecd48f7ef524a9cf166585923d91d9c72ed02cb": {
					  "707070": 18446744073709551615i128
			  }
			}
		  }
		]))
	})
	.await
	.unwrap();
	let client = HttpClient::builder().build(format!("http://{address}")).unwrap();
	let utxo = client
		.query_utxo_by_id(UtxoId::new(
			hex!("106b0d7d1544c97941777041699412fb7c8b94855210987327199620c0599580"),
			1,
		))
		.await
		.unwrap();
	assert_eq!(
		utxo.unwrap().clone(),
		OgmiosUtxo {
			transaction: OgmiosTx {
				id: hex!("106b0d7d1544c97941777041699412fb7c8b94855210987327199620c0599580")
					.try_into()
					.unwrap()
			},
			index: 1,
			address: "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy".into(),
			value: OgmiosValue {
				lovelace: 1356118,
				native_tokens: {
					let mut map = std::collections::HashMap::new();
					map.insert(
						hex!("e0d4479b3dbb53b1aecd48f7ef524a9cf166585923d91d9c72ed02cb"),
						vec![Asset {
							name: hex!("707070").to_vec(),
							amount: 18446744073709551615i128,
						}],
					);
					map
				},
			},
			datum: None,
			datum_hash: None,
			script: None
		}
	)
}
