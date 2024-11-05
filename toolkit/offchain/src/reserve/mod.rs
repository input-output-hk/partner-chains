use crate::{
	csl::*, plutus_script::PlutusScript, scripts_data::get_scripts_data,
	untyped_plutus::datum_to_uplc_plutus_data,
};
use anyhow::anyhow;
use cardano_serialization_lib::*;
use jsonrpsee::http_client::HttpClient;
use ogmios_client::{
	query_ledger_state::{ProtocolParametersResponse, QueryLedgerState},
	types::OgmiosUtxo,
};
use plutus::ToDatum;
use sidechain_domain::UtxoId;

pub async fn create_reserve<PCParams: ToDatum + Clone>(
	own_payment_vkey: [u8; 32],
	network: NetworkIdKind,
	pc_params: PCParams,
) -> anyhow::Result<()> {
	let reserve_validator = PlutusScript::from_wrapped_cbor(raw_scripts::RESERVE_VALIDATOR)?
		.apply_data(pc_params.clone())?;
	println!("validator_address: {:?}", reserve_validator.plutus_v2_address_bech32(network)?);

	let own_payment_key_hash = sidechain_domain::crypto::blake2b(&own_payment_vkey);
	println!("own_payment_key_hash: {:?}", hex::encode(own_payment_key_hash));

	let own_addr = key_ed25519_hash_address(own_payment_key_hash, network);
	println!("own_addr: {:?}", own_addr.to_bech32(None).unwrap());

	let client = HttpClient::builder().build("http://localhost:1337")?;
	let own_utxos = client.query_utxos(&[own_addr.to_bech32(None)?]).await?;

	let dummy_collateral =
		own_utxos.first().ok_or_else(|| {
			anyhow!("At least one UTXO for collateral is required when interacting with deregister script")
		})?;
	let collaterals = [dummy_collateral.clone()];

	let protocol_parameters = client.query_protocol_parameters().await?;

	let _tx = make_create_reserve_tx(
		&collaterals,
		&protocol_parameters,
		&own_addr,
		own_payment_key_hash,
		&reserve_validator,
		network,
		todo!("reserve datum"),
	);

	Ok(())
}

pub fn make_create_reserve_tx(
	collaterals: &[OgmiosUtxo],
	protocol_parameters: &ProtocolParametersResponse,
	addr: &Address,
	own_pkh: [u8; 28],
	validator: &PlutusScript,
	network: NetworkIdKind,
	reserve_datum: ReserveDatum,
) -> Result<Transaction, JsError> {
	let config = crate::csl::get_builder_config(protocol_parameters)?;
	let mut tx_builder = TransactionBuilder::new(&config);
	let tx_inputs_builder = TxInputsBuilder::new();

	//todo: add inputs
	tx_builder.set_inputs(&tx_inputs_builder);

	let mut collateral_builder = TxInputsBuilder::new();
	for collateral in collaterals.iter() {
		let amount: BigNum = crate::csl::convert_value(&collateral.value)?.coin();
		collateral_builder.add_key_input(
			&From::from(own_pkh),
			&utxo_to_tx_input(collateral),
			&Value::new(&amount),
		);
	}

	// let mut output = TransactionOutput::new(
	// 	&validator.plutus_v2_address(network),
	// 	// todo: set correct value
	// 	// n tokenów do rezerwy + 1 auth token
	// 	// + datum
	// 	&Value::zero(),
	// );
	// output.set_plutus_data(&datum_to_uplc_plutus_data(reserve_datum.to_datum()))?;
	// tx_builder.add_output(&output)?;

	tx_builder.set_collateral(&collateral_builder);
	tx_builder.calc_script_data_hash(&crate::csl::convert_cost_models(
		&protocol_parameters.plutus_cost_models,
	))?;
	tx_builder.add_required_signer(&From::from(own_pkh));
	// This should to be the last step before building the transaction
	tx_builder.add_change_if_needed(&addr)?;

	let tx: Transaction = tx_builder.build_tx()?;
	println!("Fee {:?}", tx.body().fee());
	Ok(tx)
}

async fn find_governance_versioning_utxo(
	address: &Address,
	client: HttpClient,
) -> anyhow::Result<OgmiosUtxo> {
	let own_utxos = client.query_utxos(&[address.to_bech32(None)?]).await?;

	let version_oracle_id = todo!();
	own_utxos.iter().find(|utxo| {
		utxo.value.native_tokens.get(&version_oracle_id)
			&& utxo.datum.map(|bytes| {
				let plutus_data: uplc::PlutusData = minicbor::decode(&bytes.bytes)?;
				let uplc::PlutusData::Constr(fields) = plutus_data;
				let uplc::PlutusData::BigInt(version) = fields[0];

				version == 32
			})
	});

	Ok(())
}

// [int - tylko to nas interesuje, currency symbol ☝️]
// data VersionOracleDatum = VersionOracleDatum
// { versionOracle :: VersionOracle
// -- ^ VersionOracle which identifies the script.
// -- @since v6.0.0
// , currencySymbol :: CurrencySymbol
// -- ^ Currency Symbol of the VersioningOraclePolicy tokens.
// -- @since v6.0.0
// }

pub struct VersionOracleDatum {
	script_number: u32,
	currency_symbol: String,
}

// #[derive(ToDatum)]
pub struct ReserveDatum {
	//dupa
}
