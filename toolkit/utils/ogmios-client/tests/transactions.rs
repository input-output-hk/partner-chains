#![cfg(feature = "jsonrpsee-client")]

use hex_literal::hex;
use jsonrpsee::types::ErrorCode;
use ogmios_client::{
	jsonrpsee::client_for_url,
	transactions::{
		OgmiosBudget, OgmiosEvaluateTransactionResponse, OgmiosValidatorIndex,
		SubmitTransactionResponse, Transactions,
	},
	types::OgmiosTx,
};
use serde_json::json;

mod server;

#[tokio::test]
async fn evaluate_transaction() {
	let address = server::for_single_test("evaluateTransaction", |req| {
		let expected_params = json!({"transaction": { "cbor": "aabbccdd" }, "additionalUtxo": []});
		let params_value: serde_json::Value = req.parse().unwrap();
		if params_value == expected_params {
			Ok(json!([{
			  "validator": {
				"index": 0,
				"purpose": "spend"
			  },
			  "budget": {
				"memory": 202586,
				"cpu": 43653414
			  }
			}]))
		} else {
			Err(ErrorCode::InvalidParams.into())
		}
	})
	.await
	.unwrap();
	let client = client_for_url(&format!("http://{address}")).await.unwrap();
	let response = client.evaluate_transaction(&hex!("aabbccdd")).await.unwrap();
	assert_eq!(
		response[0],
		OgmiosEvaluateTransactionResponse {
			validator: OgmiosValidatorIndex { index: 0, purpose: "spend".into() },
			budget: OgmiosBudget { memory: 202586, cpu: 43653414 },
		}
	)
}

#[tokio::test]
async fn submit_transaction() {
	let address = server::for_single_test("submitTransaction", |req| {
    	let expected_params = json!({"transaction": { "cbor": "aabbccdd" }});
    	let params_value: serde_json::Value = req.parse().unwrap();
    	if params_value == expected_params {
    		Ok(json!({"transaction": { "id": "e4891cd4e45c320301fff691bbd1bee0cf4484fc2ddc26c08c555d08efbb7d6b" }}))
    	} else {
    		Err(ErrorCode::InvalidParams.into())
    	}
	})
	.await
	.unwrap();
	let client = client_for_url(&format!("http://{address}")).await.unwrap();
	let response = client.submit_transaction(&hex!("aabbccdd")).await.unwrap();
	assert_eq!(
		response,
		SubmitTransactionResponse {
			transaction: OgmiosTx {
				id: hex!("e4891cd4e45c320301fff691bbd1bee0cf4484fc2ddc26c08c555d08efbb7d6b")
			}
		}
	)
}
