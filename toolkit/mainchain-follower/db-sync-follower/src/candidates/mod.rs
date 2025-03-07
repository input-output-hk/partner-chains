use crate::db_model::{
	self, Address, Asset, BlockNumber, EpochNumber, MainchainTxOutput, StakePoolEntry,
};
use crate::metrics::McFollowerMetrics;
use crate::observed_async_trait;
use crate::DataSourceError::*;
use authority_selection_inherents::authority_selection_inputs::*;
use datum::raw_permissioned_candidate_data_vec_from;
use itertools::Itertools;
use log::error;
use partner_chains_plutus_data::{
	d_param::DParamDatum, permissioned_candidates::PermissionedCandidateDatums,
	registered_candidates::RegisterValidatorDatum,
};
use sidechain_domain::*;
use sqlx::PgPool;
use std::collections::HashMap;
use std::error::Error;

pub mod cached;
pub mod datum;

#[cfg(test)]
pub mod tests;

#[derive(Clone, Debug)]
struct ParsedCandidate {
	utxo_info: UtxoInfo,
	datum: RegisterValidatorDatum,
	tx_inputs: Vec<UtxoId>,
}

#[derive(Debug)]
struct RegisteredCandidate {
	stake_pool_pub_key: StakePoolPublicKey,
	registration_utxo: UtxoId,
	tx_inputs: Vec<UtxoId>,
	sidechain_signature: SidechainSignature,
	mainchain_signature: MainchainSignature,
	cross_chain_signature: CrossChainSignature,
	sidechain_pub_key: SidechainPublicKey,
	cross_chain_pub_key: CrossChainPublicKey,
	aura_pub_key: AuraPublicKey,
	grandpa_pub_key: GrandpaPublicKey,
	utxo_info: UtxoInfo,
}

pub struct CandidatesDataSourceImpl {
	pool: PgPool,
	metrics_opt: Option<McFollowerMetrics>,
}

observed_async_trait!(
impl AuthoritySelectionDataSource for CandidatesDataSourceImpl {
	async fn get_ariadne_parameters(
			&self,
			epoch: McEpochNumber,
			d_parameter_policy: PolicyId,
			permissioned_candidate_policy: PolicyId
	) -> Result<AriadneParameters, Box<dyn std::error::Error + Send + Sync>> {
		let epoch = EpochNumber::from(self.get_epoch_of_data_storage(epoch)?);
		let d_parameter_asset = Asset::new(d_parameter_policy);
		let permissioned_candidate_asset = Asset::new(permissioned_candidate_policy);

		let (candidates_output_opt, d_output_opt) = tokio::try_join!(
			db_model::get_token_utxo_for_epoch(&self.pool, &permissioned_candidate_asset, epoch),
			db_model::get_token_utxo_for_epoch(&self.pool, &d_parameter_asset, epoch)
		)?;

		let d_output = d_output_opt.ok_or(ExpectedDataNotFound("DParameter".to_string()))?;

		let d_datum = d_output
			.datum
			.map(|d| d.0)
			.ok_or(ExpectedDataNotFound("DParameter Datum".to_string()))?;

		let d_parameter = DParamDatum::try_from(d_datum)?.into();

		let candidates_output = candidates_output_opt
			.ok_or(ExpectedDataNotFound("Permissioned Candidates List".to_string()))?;

		let candidates_datum = candidates_output
			.datum
			.map(|d| d.0)
			.ok_or(ExpectedDataNotFound("Permissioned Candidates List Datum".to_string()))?;

		let permissioned_candidates = raw_permissioned_candidate_data_vec_from(
			PermissionedCandidateDatums::try_from(candidates_datum)?
		);

		Ok(AriadneParameters { d_parameter, permissioned_candidates })
	}

	async fn get_candidates(
			&self,
			epoch: McEpochNumber,
			committee_candidate_address: MainchainAddress
	)-> Result<Vec<CandidateRegistrations>, Box<dyn std::error::Error + Send + Sync>> {
		let epoch = EpochNumber::from(self.get_epoch_of_data_storage(epoch)?);
		let candidates = self.get_registered_candidates(epoch, committee_candidate_address).await?;
		let stake_map = Self::make_stake_map(db_model::get_stake_distribution(&self.pool, epoch).await?);
		Ok(Self::group_candidates_by_mc_pub_key(candidates).into_iter().map(|(mainchain_pub_key, candidate_registrations)| {
			CandidateRegistrations {
				stake_pool_public_key: mainchain_pub_key.clone(),
				registrations: candidate_registrations.into_iter().map(Self::make_registration_data).collect(),
				stake_delegation: Self::get_stake_delegation(&stake_map, &mainchain_pub_key),
			}
		}).collect())
	}

	async fn get_epoch_nonce(&self, epoch: McEpochNumber) -> Result<Option<EpochNonce>, Box<dyn std::error::Error + Send + Sync>> {
		let epoch = self.get_epoch_of_data_storage(epoch)?;
		let nonce = db_model::get_epoch_nonce(&self.pool, EpochNumber(epoch.0)).await?;
		Ok(nonce.map(|n| EpochNonce(n.0)))
	}

	async fn data_epoch(&self, for_epoch: McEpochNumber) -> Result<McEpochNumber, Box<dyn std::error::Error + Send + Sync>> {
		self.get_epoch_of_data_storage(for_epoch)
	}
});

impl CandidatesDataSourceImpl {
	pub async fn new(
		pool: PgPool,
		metrics_opt: Option<McFollowerMetrics>,
	) -> Result<CandidatesDataSourceImpl, Box<dyn std::error::Error + Send + Sync>> {
		db_model::create_idx_ma_tx_out_ident(&pool).await?;
		Ok(Self { pool, metrics_opt })
	}

	pub fn cached(
		self,
		candidates_for_epoch_cache_size: usize,
	) -> std::result::Result<cached::CandidateDataSourceCached, Box<dyn Error + Send + Sync>> {
		cached::CandidateDataSourceCached::new_from_env(self, candidates_for_epoch_cache_size)
	}

	/// Registrations state up to this block are considered as "active", after it - as "pending".
	async fn get_last_block_for_epoch(
		&self,
		epoch: EpochNumber,
	) -> Result<Option<BlockNumber>, Box<dyn std::error::Error + Send + Sync>> {
		let block_option = db_model::get_latest_block_for_epoch(&self.pool, epoch).await?;
		Ok(block_option.map(|b| b.block_no))
	}

	async fn get_registered_candidates(
		&self,
		epoch: EpochNumber,
		committee_candidate_address: MainchainAddress,
	) -> Result<Vec<RegisteredCandidate>, Box<dyn std::error::Error + Send + Sync>> {
		let registrations_block_for_epoch = self.get_last_block_for_epoch(epoch).await?;
		let address: Address = Address(committee_candidate_address.to_string());
		let active_utxos = match registrations_block_for_epoch {
			Some(block) => db_model::get_utxos_for_address(&self.pool, &address, block).await?,
			None => vec![],
		};
		self.convert_utxos_to_candidates(&active_utxos)
	}

	fn group_candidates_by_mc_pub_key(
		candidates: Vec<RegisteredCandidate>,
	) -> HashMap<StakePoolPublicKey, Vec<RegisteredCandidate>> {
		candidates.into_iter().into_group_map_by(|c| c.stake_pool_pub_key.clone())
	}

	fn make_registration_data(c: RegisteredCandidate) -> RegistrationData {
		RegistrationData {
			registration_utxo: c.registration_utxo,
			sidechain_signature: c.sidechain_signature,
			mainchain_signature: c.mainchain_signature,
			cross_chain_signature: c.cross_chain_signature,
			sidechain_pub_key: c.sidechain_pub_key,
			cross_chain_pub_key: c.cross_chain_pub_key,
			aura_pub_key: c.aura_pub_key,
			grandpa_pub_key: c.grandpa_pub_key,
			utxo_info: c.utxo_info,
			tx_inputs: c.tx_inputs,
		}
	}

	fn make_stake_map(
		stake_pool_entries: Vec<StakePoolEntry>,
	) -> HashMap<MainchainKeyHash, StakeDelegation> {
		stake_pool_entries
			.into_iter()
			.map(|e| (MainchainKeyHash(e.pool_hash), StakeDelegation(e.stake.0)))
			.collect()
	}

	fn get_stake_delegation(
		stake_map: &HashMap<MainchainKeyHash, StakeDelegation>,
		stake_pool_pub_key: &StakePoolPublicKey,
	) -> Option<StakeDelegation> {
		if stake_map.is_empty() {
			None
		} else {
			Some(
				stake_map
					.get(&MainchainKeyHash::from_vkey(&stake_pool_pub_key.0))
					.cloned()
					.unwrap_or(StakeDelegation(0)),
			)
		}
	}

	// Converters
	fn convert_utxos_to_candidates(
		&self,
		outputs: &[MainchainTxOutput],
	) -> Result<Vec<RegisteredCandidate>, Box<dyn std::error::Error + Send + Sync>> {
		Self::parse_candidates(outputs)
			.into_iter()
			.map(|c| {
				let RegisterValidatorDatum::V0 {
					stake_ownership,
					sidechain_pub_key,
					sidechain_signature,
					registration_utxo,
					own_pkh: _own_pkh,
					aura_pub_key,
					grandpa_pub_key,
				} = c.datum;
				Ok(RegisteredCandidate {
					stake_pool_pub_key: stake_ownership.pub_key,
					mainchain_signature: stake_ownership.signature,
					// For now we use the same key for both cross chain and sidechain actions
					cross_chain_pub_key: CrossChainPublicKey(sidechain_pub_key.0.clone()),
					cross_chain_signature: CrossChainSignature(sidechain_signature.0.clone()),
					sidechain_signature,
					sidechain_pub_key,
					aura_pub_key,
					grandpa_pub_key,
					registration_utxo,
					tx_inputs: c.tx_inputs,
					utxo_info: c.utxo_info,
				})
			})
			.collect()
	}

	fn parse_candidates(outputs: &[MainchainTxOutput]) -> Vec<ParsedCandidate> {
		let results: Vec<std::result::Result<ParsedCandidate, String>> = outputs
			.iter()
			.map(|output| {
				let datum = output.datum.clone().ok_or(format!(
					"Missing registration datum for {:?}",
					output.clone().utxo_id
				))?;
				let register_validator_datum =
					RegisterValidatorDatum::try_from(datum).map_err(|_| {
						format!("Invalid registration datum for {:?}", output.clone().utxo_id)
					})?;
				Ok(ParsedCandidate {
					utxo_info: UtxoInfo {
						utxo_id: output.utxo_id,
						epoch_number: output.tx_epoch_no.into(),
						block_number: output.tx_block_no.into(),
						slot_number: output.tx_slot_no.into(),
						tx_index_within_block: McTxIndexInBlock(output.tx_index_in_block.0),
					},
					datum: register_validator_datum,
					tx_inputs: output.tx_inputs.clone(),
				})
			})
			.collect();
		results
			.into_iter()
			.filter_map(|r| match r {
				Ok(candidate) => Some(candidate.clone()),
				Err(msg) => {
					error!("{msg}");
					None
				},
			})
			.collect()
	}

	fn get_epoch_of_data_storage(
		&self,
		epoch_of_data_usage: McEpochNumber,
	) -> Result<McEpochNumber, Box<dyn std::error::Error + Send + Sync>> {
		offset_data_epoch(epoch_of_data_usage).map_err(|offset| {
			BadRequest(format!(
				"Minimum supported epoch of data usage is {offset}, but {} was provided",
				epoch_of_data_usage.0
			))
			.into()
		})
	}
}
