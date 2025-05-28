use partner_chains_cardano_offchain::{
	await_tx::FixedDelayRetries, cardano_keys::CardanoPaymentSigningKey,
	governance::MultiSigParameters, multisig::MultiSigSmartContractResult,
	scripts_data::ScriptsData,
};
use sidechain_domain::{
	CandidateRegistration, DParameter, McTxHash, PermissionedCandidateData, StakePoolPublicKey,
	UtxoId,
};

/// Initializes governance mechanism.
pub trait InitGovernance {
	/// Initializes governance mechanism with Cardano Native Script of type `atLeast` parametrized with values from
	/// `governance_parameters`, for the chain identified by `genesis_utxo_id`.
	#[allow(async_fn_in_trait)]
	async fn init_governance(
		&self,
		await_tx: FixedDelayRetries,
		governance_parameters: &MultiSigParameters,
		payment_key: &CardanoPaymentSigningKey,
		genesis_utxo_id: UtxoId,
	) -> Result<McTxHash, String>;
}

/// For the given `genesis_utxo` it returns the [ScriptsData] of the partner chain smart contracts.
pub trait GetScriptsData {
	#[allow(async_fn_in_trait)]
	/// For the given `genesis_utxo` it returns the [ScriptsData] of the partner chain smart contracts.
	async fn get_scripts_data(&self, genesis_utxo: UtxoId) -> Result<ScriptsData, String>;
}

/// Upserts D-param.
pub trait UpsertDParam {
	#[allow(async_fn_in_trait)]
	/// This function upserts D-param.
	async fn upsert_d_param(
		&self,
		await_tx: FixedDelayRetries,
		genesis_utxo: UtxoId,
		d_parameter: &DParameter,
		payment_signing_key: &CardanoPaymentSigningKey,
	) -> anyhow::Result<Option<MultiSigSmartContractResult>>;
}
/// Returns D-parameter.
pub trait GetDParam {
	#[allow(async_fn_in_trait)]
	/// Returns D-parameter.
	async fn get_d_param(&self, genesis_utxo: UtxoId) -> anyhow::Result<Option<DParameter>>;
}

/// Registers a registered candidate.
pub trait Register {
	#[allow(async_fn_in_trait)]
	/// This function submits a transaction to register a registered candidate.
	async fn register(
		&self,
		await_tx: FixedDelayRetries,
		genesis_utxo: UtxoId,
		candidate_registration: &CandidateRegistration,
		payment_signing_key: &CardanoPaymentSigningKey,
	) -> Result<Option<McTxHash>, String>;
}

/// Deregisters a registered candidate.
pub trait Deregister {
	#[allow(async_fn_in_trait)]
	/// This function submits a transaction to deregister a registered candidate.
	async fn deregister(
		&self,
		await_tx: FixedDelayRetries,
		genesis_utxo: UtxoId,
		payment_signing_key: &CardanoPaymentSigningKey,
		stake_ownership_pub_key: StakePoolPublicKey,
	) -> Result<Option<McTxHash>, String>;
}

/// Upserts permissioned candidates list.
pub trait UpsertPermissionedCandidates {
	#[allow(async_fn_in_trait)]
	/// Upserts permissioned candidates list.
	async fn upsert_permissioned_candidates(
		&self,
		await_tx: FixedDelayRetries,
		genesis_utxo: UtxoId,
		candidates: &[PermissionedCandidateData],
		payment_signing_key: &CardanoPaymentSigningKey,
	) -> anyhow::Result<Option<MultiSigSmartContractResult>>;
}

/// Returns all permissioned candidates.
pub trait GetPermissionedCandidates {
	#[allow(async_fn_in_trait)]
	/// Returns all permissioned candidates.
	async fn get_permissioned_candidates(
		&self,
		genesis_utxo: UtxoId,
	) -> anyhow::Result<Option<Vec<PermissionedCandidateData>>>;
}
