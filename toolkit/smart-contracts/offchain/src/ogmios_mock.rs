use ogmios_client::{
	query_ledger_state::{
		OgmiosTip, ProtocolParametersResponse, QueryLedgerState, QueryUtxoByUtxoId,
	},
	query_network::{QueryNetwork, ShelleyGenesisConfigurationResponse},
	transactions::{OgmiosEvaluateTransactionResponse, SubmitTransactionResponse, Transactions},
	types::OgmiosUtxo,
	OgmiosClientError,
};

#[derive(Clone, Default, Debug)]
pub struct MockOgmiosClient {
	shelley_config: ShelleyGenesisConfigurationResponse,
	utxos: Vec<OgmiosUtxo>,
	protocol_parameters: ProtocolParametersResponse,
	evaluate_result: Option<Vec<OgmiosEvaluateTransactionResponse>>,
	submit_result: Option<SubmitTransactionResponse>,
}

impl MockOgmiosClient {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_utxos(self, utxos: Vec<OgmiosUtxo>) -> Self {
		let mut current_utxos = self.utxos;
		current_utxos.extend(utxos);
		Self { utxos: current_utxos, ..self }
	}

	pub fn with_protocol_parameters(self, protocol_parameters: ProtocolParametersResponse) -> Self {
		Self { protocol_parameters, ..self }
	}

	pub fn with_evaluate_result(
		self,
		evaluate_result: Vec<OgmiosEvaluateTransactionResponse>,
	) -> Self {
		Self { evaluate_result: Some(evaluate_result), ..self }
	}

	pub fn with_submit_result(self, submit_result: SubmitTransactionResponse) -> Self {
		Self { submit_result: Some(submit_result), ..self }
	}

	pub fn with_shelley_config(self, shelley_config: ShelleyGenesisConfigurationResponse) -> Self {
		Self { shelley_config, ..self }
	}
}

impl QueryNetwork for MockOgmiosClient {
	async fn shelley_genesis_configuration(
		&self,
	) -> Result<ShelleyGenesisConfigurationResponse, ogmios_client::OgmiosClientError> {
		Ok(self.shelley_config.clone())
	}
}

impl Transactions for MockOgmiosClient {
	async fn evaluate_transaction(
		&self,
		_tx_bytes: &[u8],
	) -> Result<
		Vec<ogmios_client::transactions::OgmiosEvaluateTransactionResponse>,
		ogmios_client::OgmiosClientError,
	> {
		Ok(self.evaluate_result.clone().unwrap())
	}

	async fn submit_transaction(
		&self,
		_tx_bytes: &[u8],
	) -> Result<
		ogmios_client::transactions::SubmitTransactionResponse,
		ogmios_client::OgmiosClientError,
	> {
		Ok(self.submit_result.clone().unwrap())
	}
}

impl QueryLedgerState for MockOgmiosClient {
	async fn get_tip(&self) -> Result<OgmiosTip, OgmiosClientError> {
		unimplemented!()
	}

	async fn era_summaries(
		&self,
	) -> Result<Vec<ogmios_client::query_ledger_state::EraSummary>, ogmios_client::OgmiosClientError>
	{
		unimplemented!()
	}

	async fn query_utxos(
		&self,
		addresses: &[String],
	) -> Result<Vec<ogmios_client::types::OgmiosUtxo>, ogmios_client::OgmiosClientError> {
		Ok(self
			.utxos
			.iter()
			.filter(|utxo| addresses.contains(&utxo.address))
			.cloned()
			.collect())
	}

	async fn query_protocol_parameters(
		&self,
	) -> Result<
		ogmios_client::query_ledger_state::ProtocolParametersResponse,
		ogmios_client::OgmiosClientError,
	> {
		Ok(self.protocol_parameters.clone())
	}
}

impl QueryUtxoByUtxoId for MockOgmiosClient {
	async fn query_utxo_by_id(
		&self,
		queried_utxo: sidechain_domain::UtxoId,
	) -> Result<Option<OgmiosUtxo>, ogmios_client::OgmiosClientError> {
		Ok(self.utxos.iter().find(|utxo| utxo.utxo_id() == queried_utxo).cloned())
	}
}
