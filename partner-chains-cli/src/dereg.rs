use crate::CmdRun;
use crate::IOContext;
use hex_literal::hex;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::params::ObjectParams;
use jsonrpsee::core::traits::ToRpcParams;
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::rpc_params;
use pallas_addresses::{ShelleyAddress, ShelleyDelegationPart, ShelleyPaymentPart};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, clap::Parser)]
pub struct DeregCmd;

impl CmdRun for DeregCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		let _ = tokio::task::block_in_place(|| {
			tokio::runtime::Runtime::new().unwrap().block_on(async_run(context))
		});
		Ok(())
	}
}

async fn async_run<C: IOContext>(context: &C) -> anyhow::Result<()> {
	let own_payment_vkey = hex!("a35ef86f1622172816bb9e916aea86903b2c8d32c728ad5c9b9472be7e3c5e88");
	let own_payment_key_hash = sidechain_domain::MainchainAddressHash::from_vkey(own_payment_vkey);
	let own_addr = ShelleyAddress::new(
		pallas_addresses::Network::Testnet,
		ShelleyPaymentPart::key_hash(own_payment_key_hash.0.into()),
		ShelleyDelegationPart::Null,
	);
	//addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy
	println!("own_phk: {:?}, own_addr: {:?}", own_payment_key_hash, own_addr.to_bech32().unwrap());

	let client = HttpClient::builder().build("http://localhost:1337")?;
	let mut params = ObjectParams::new();
	params
		.insert(
			"addresses",
			vec!["addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy"],
		)
		.unwrap();

	context.print(&format!("Params: {}", params.clone().to_rpc_params().unwrap().unwrap()));
	let response: Result<Vec<OgmiosUtxo>, _> =
		client.request("queryLedgerState/utxo", params).await;
	context.print(&format!("Response: {:?}", response));
	Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OgmiosUtxoQueryParams {
	addresses: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OgmiosUtxo {
	transaction: OgmiosTx,
	index: u32,
	address: String,
	value: OgmiosValue,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OgmiosTx {
	id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum OgmiosValue {
	ada { lovelace: u64 },
}
/*

{
  "jsonrpc": "2.0",
  "method": "queryLedgerState/utxo",
  "result": [
	{
	  "transaction": {
		"id": "05f5c45e4cf23feb2d6bfd1e56afaba12b88fdbc75f3e1e9fd3bd1c594c67c92"
	  },
	  "index": 1,
	  "address": "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
	  "value": {
		"ada": {
		  "lovelace": 9988266569
		}
	  }
	},
	{
	  "transaction": {
		"id": "0aaf396318cc2b065c9bde43465bd17500bb60faab9f65b5c247bed977fdf9de"
	  },
	  "index": 1,
	  "address": "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
	  "value": {
		"ada": {
		  "lovelace": 1583712
		}
	  }
	},
	{
	  "transaction": {
		"id": "480636be3323ba73606b720ec44beabcd4b68be07ab3cd8b077ad616927ca937"
	  },
	  "index": 1,
	  "address": "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
	  "value": {
		"ada": {
		  "lovelace": 1583712
		}
	  }
	},
	{
	  "transaction": {
		"id": "6785e0befaafc2af2f39de5753214baaddf6428902e1b47e573595bb6024160c"
	  },
	  "index": 1,
	  "address": "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
	  "value": {
		"ada": {
		  "lovelace": 1321285
		}
	  }
	},
	{
	  "transaction": {
		"id": "716c385673909c5235853fa43ba0930595acac7e006afaaf87d47135ef5d02fc"
	  },
	  "index": 1,
	  "address": "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
	  "value": {
		"ada": {
		  "lovelace": 9990535356
		}
	  }
	},
	{
	  "transaction": {
		"id": "79d3afe95d9b3c5b444bb38517d7f2333806b33747bf158b4ed23884cf018072"
	  },
	  "index": 1,
	  "address": "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
	  "value": {
		"ada": {
		  "lovelace": 1367434
		}
	  }
	}
  ],
  "id": null
}

*/
/*
deregister (DeregisterParams { sidechainParams, spoPubKey }) = do
  ...
  validator <- getCommitteeCandidateValidator sidechainParams
  valAddr <- toAddress (PlutusScript.hash validator)
  ownUtxos <- Effect.utxosAt ownAddr
  valUtxos <- Effect.utxosAt valAddr

  { ownRegistrationUtxos } <- findOwnRegistrations ownPkh spoPubKey valUtxos

  when (null ownRegistrationUtxos)
	$ throw
		(NotFoundInputUtxo "Couldn't find registration UTxO")

  let
	lookups :: Lookups.ScriptLookups
	lookups = Lookups.validator validator
	  <> Lookups.unspentOutputs ownUtxos
	  <> Lookups.unspentOutputs valUtxos

	constraints :: Constraints.TxConstraints
	constraints = Constraints.mustBeSignedBy ownPkh
	  <> mconcat
		( flip Constraints.mustSpendScriptOutput (RedeemerDatum unit) <$>
			ownRegistrationUtxos
		)

  balanceSignAndSubmit "Deregister Committee Candidate" { lookups, constraints }

-- | Based on the wallet public key hash and the SPO public key, it finds the
-- | the registration UTxOs of the committee member/candidate
findOwnRegistrations ::
  forall r.
  PaymentPubKeyHash ->
  Maybe PubKey ->
  UtxoMap ->
  Run r
	{ ownRegistrationUtxos :: Array TransactionInput
	, ownRegistrationDatums :: Array BlockProducerRegistration
	}
findOwnRegistrations ownPkh spoPubKey validatorUtxos = do
  mayTxInsAndBlockProducerRegistrations <- Map.toUnfoldable validatorUtxos #
	traverse
	  \(input /\ TransactionOutput out) ->
		pure do
		  d <- outputDatumDatum =<< out.datum
		  BlockProducerRegistration r <- fromData d
		  guard
			( (getSPOPubKey r.stakeOwnership == spoPubKey) &&
				(r.ownPkh == ownPkh)
			)
		  pure (input /\ BlockProducerRegistration r)

  let
	txInsAndBlockProducerRegistrations = catMaybes
	  mayTxInsAndBlockProducerRegistrations
	ownRegistrationUtxos = map fst txInsAndBlockProducerRegistrations
	ownRegistrationDatums = map snd txInsAndBlockProducerRegistrations
  pure $ { ownRegistrationUtxos, ownRegistrationDatums }

*/
