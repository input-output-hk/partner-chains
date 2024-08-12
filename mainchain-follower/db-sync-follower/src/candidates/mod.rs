use crate::db_model::{
	self, Address, Asset, BlockNumber, EpochNumber, MainchainTxOutput, StakePoolEntry,
};
use crate::metrics::McFollowerMetrics;
use crate::observed_async_trait;
use async_trait::async_trait;
use itertools::Itertools;
use log::error;
use main_chain_follower_api::candidate::{AriadneParameters, RawPermissionedCandidateData};
use main_chain_follower_api::{CandidateDataSource, DataSourceError::*, Result};
use num_traits::ToPrimitive;
use plutus::Datum;
use plutus::Datum::*;
use sidechain_domain::*;
use sqlx::PgPool;
use std::collections::HashMap;

pub mod cached;

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
	mainchain_pub_key: MainchainPublicKey,
	consumed_input: UtxoId,
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

/** Representation of the plutus type in the mainchain contract (rev 4ed2cc66c554ec8c5bec7b90ad9273e9069a1fb4)
*
* Note that the ECDSA secp256k1 public key is serialized in compressed format and the
* sidechain signature does not contain the recovery bytes (it's just r an s concatenated).
*
* data BlockProducerRegistration = BlockProducerRegistration
* { -- | Verification keys required by the stake ownership model
*   -- | @since v4.0.0
*  stakeOwnership :: StakeOwnership
* , -- | public key in the sidechain's desired format
*  sidechainPubKey :: LedgerBytes
* , -- | Signature of the sidechain
*   -- | @since v4.0.0
*  sidechainSignature :: Signature
* , -- | A UTxO that must be spent by the transaction
*   -- | @since v4.0.0
*  inputUtxo :: TxOutRef
* , -- | Owner public key hash
*   -- | @since v4.0.0
*  ownPkh :: PubKeyHash
* , -- | Sidechain authority discovery key
*   -- | @since Unreleased
*   auraKey :: LedgerBytes
* , -- | Sidechain grandpa key
*   -- | @since Unreleased
*   grandpaKey :: LedgerBytes
* }
 */
#[derive(Clone, Debug)]
struct RegisterValidatorDatum {
	stake_ownership: AdaBasedStaking,
	sidechain_pub_key: SidechainPublicKey,
	sidechain_signature: SidechainSignature,
	consumed_input: UtxoId,
	//ownPkh we don't use,
	aura_pub_key: AuraPublicKey,
	grandpa_pub_key: GrandpaPublicKey,
}

/// AdaBasedStaking is a variant of Plutus type StakeOwnership.
/// The other variant, TokenBasedStaking, is not supported
#[derive(Clone, Debug)]
struct AdaBasedStaking {
	pub_key: MainchainPublicKey,
	signature: MainchainSignature,
}

pub struct CandidatesDataSourceImpl {
	pool: PgPool,
	metrics_opt: Option<McFollowerMetrics>,
}

observed_async_trait!(
impl CandidateDataSource for CandidatesDataSourceImpl {
	async fn get_ariadne_parameters(
			&self,
			epoch: McEpochNumber,
			d_parameter_policy: PolicyId,
			permissioned_candidate_policy: PolicyId
		) -> Result<AriadneParameters> {
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

		let d_param = Self::decode_d_parameter_datum(&d_datum)?;

		let candidates_output = candidates_output_opt
			.ok_or(ExpectedDataNotFound("Permissioned Candidates List".to_string()))?;

		let candidates_datum = candidates_output
			.datum
			.map(|d| d.0)
			.ok_or(ExpectedDataNotFound("Permissioned Candidates List Datum".to_string()))?;

		let candidates = Self::decode_permissioned_candidates_datum(&candidates_datum)?;

		Ok(AriadneParameters { d_parameter: d_param, permissioned_candidates: candidates })
	}

	async fn get_candidates(
			&self,
			epoch: McEpochNumber,
			committee_candidate_address: MainchainAddress
		)-> Result<Vec<CandidateRegistrations>> {
		let epoch = EpochNumber::from(self.get_epoch_of_data_storage(epoch)?);
		let candidates = self.get_registered_candidates(epoch, committee_candidate_address).await?;
		let stake_map = Self::make_stake_map(db_model::get_stake_distribution(&self.pool, epoch).await?);
		Ok(Self::group_candidates_by_mc_pub_key(candidates).into_iter().map(|(mainchain_pub_key, candidate_registrations)| {
			CandidateRegistrations {
				mainchain_pub_key: mainchain_pub_key.clone(),
				registrations: candidate_registrations.into_iter().map(Self::make_registration_data).collect(),
				stake_delegation: Self::get_stake_delegation(&stake_map, &mainchain_pub_key),
			}
		}).collect())
	}

	async fn get_epoch_nonce(&self, epoch: McEpochNumber) -> Result<Option<EpochNonce>> {
		let epoch = self.get_epoch_of_data_storage(epoch)?;
		let nonce = db_model::get_epoch_nonce(&self.pool, EpochNumber(epoch.0)).await?;
		Ok(nonce.map(|n| EpochNonce(n.0)))
	}

	async fn data_epoch(&self, for_epoch: McEpochNumber) -> Result<McEpochNumber> {
		self.get_epoch_of_data_storage(for_epoch)
	}
});

impl CandidatesDataSourceImpl {
	pub(crate) fn new(pool: PgPool, metrics_opt: Option<McFollowerMetrics>) -> Self {
		Self { pool, metrics_opt }
	}

	pub async fn from_config(
		pool: PgPool,
		metrics_opt: Option<McFollowerMetrics>,
	) -> Result<CandidatesDataSourceImpl> {
		db_model::create_idx_ma_tx_out_ident(&pool).await?;
		Ok(CandidatesDataSourceImpl::new(pool, metrics_opt))
	}

	/// Registrations state up to this block are considered as "active", after it - as "pending".
	async fn get_last_block_for_epoch(&self, epoch: EpochNumber) -> Result<Option<BlockNumber>> {
		let block_option = db_model::get_latest_block_for_epoch(&self.pool, epoch).await?;
		Ok(block_option.map(|b| b.block_no))
	}

	async fn get_registered_candidates(
		&self,
		epoch: EpochNumber,
		committee_candidate_address: MainchainAddress,
	) -> Result<Vec<RegisteredCandidate>> {
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
	) -> HashMap<MainchainPublicKey, Vec<RegisteredCandidate>> {
		candidates.into_iter().into_group_map_by(|c| c.mainchain_pub_key.clone())
	}

	fn make_registration_data(c: RegisteredCandidate) -> RegistrationData {
		RegistrationData {
			consumed_input: c.consumed_input,
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
	) -> HashMap<MainchainAddressHash, StakeDelegation> {
		stake_pool_entries
			.into_iter()
			.map(|e| (MainchainAddressHash(e.pool_hash), StakeDelegation(e.stake.0)))
			.collect()
	}

	fn get_stake_delegation(
		stake_map: &HashMap<MainchainAddressHash, StakeDelegation>,
		mainchain_pub_key: &MainchainPublicKey,
	) -> Option<StakeDelegation> {
		if stake_map.is_empty() {
			None
		} else {
			Some(
				stake_map
					.get(&MainchainAddressHash::from_vkey(mainchain_pub_key.0))
					.cloned()
					.unwrap_or(StakeDelegation(0)),
			)
		}
	}

	// Converters
	fn convert_utxos_to_candidates(
		&self,
		outputs: &[MainchainTxOutput],
	) -> Result<Vec<RegisteredCandidate>> {
		Self::parse_candidates(outputs)
			.into_iter()
			.map(|c| {
				Ok(RegisteredCandidate {
					mainchain_pub_key: c.datum.stake_ownership.pub_key,
					mainchain_signature: c.datum.stake_ownership.signature,
					// For now we use the same key for both cross chain and sidechain actions
					cross_chain_pub_key: CrossChainPublicKey(c.datum.sidechain_pub_key.0.clone()),
					cross_chain_signature: CrossChainSignature(
						c.datum.sidechain_signature.0.clone(),
					),
					sidechain_signature: c.datum.sidechain_signature,
					sidechain_pub_key: c.datum.sidechain_pub_key,
					aura_pub_key: c.datum.aura_pub_key,
					grandpa_pub_key: c.datum.grandpa_pub_key,
					consumed_input: c.datum.consumed_input,
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
				let register_validator_datum = Self::decode_register_validator_datum(&datum)
					.ok_or(format!(
						"Invalid registration datum for {:?}",
						output.clone().utxo_id
					))?;
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

	// Datum decoders
	fn decode_d_parameter_datum(datum: &Datum) -> Result<DParameter> {
		let d_parameter = match datum {
			ListDatum(items) => match items.first().zip(items.get(1)) {
				Some((IntegerDatum(p), IntegerDatum(t))) => {
					p.to_u16().zip(t.to_u16()).map(|(p, t)| DParameter {
						num_permissioned_candidates: p,
						num_registered_candidates: t,
					})
				},
				_ => None,
			},
			_ => None,
		}
		.ok_or(DatumDecodeError { datum: datum.clone(), to: "DParameter".to_string() });
		if d_parameter.is_err() {
			error!("Could not decode {:?} to DParameter. Expected [u16, u16].", datum.clone());
		}
		d_parameter
	}

	fn decode_permissioned_candidates_datum(
		datum: &Datum,
	) -> Result<Vec<RawPermissionedCandidateData>> {
		let permissioned_candidates: Result<Vec<RawPermissionedCandidateData>> = match datum {
			ListDatum(list_datums) => list_datums
				.iter()
				.map(|keys_datum| match keys_datum {
					ListDatum(d) => {
						let sc = d.first().and_then(|d| d.as_bytestring())?;
						let aura = d.get(1).and_then(|d| d.as_bytestring())?;
						let grandpa = d.get(2).and_then(|d| d.as_bytestring())?;
						Some(RawPermissionedCandidateData {
							sidechain_public_key: SidechainPublicKey(sc.clone()),
							aura_public_key: AuraPublicKey(aura.clone()),
							grandpa_public_key: GrandpaPublicKey(grandpa.clone()),
						})
					},
					_ => None,
				})
				.collect::<Option<Vec<RawPermissionedCandidateData>>>(),
			_ => None,
		}
		.ok_or(DatumDecodeError {
			datum: datum.clone(),
			to: "RawPermissionedCandidateData".to_string(),
		});

		if permissioned_candidates.is_err() {
			error!("Could not decode {:?} to Permissioned candidates datum. Expected [[ByteString, ByteString, ByteString]].", datum.clone());
		}
		permissioned_candidates
	}

	fn decode_register_validator_datum(datum: &Datum) -> Option<RegisterValidatorDatum> {
		match datum {
			ConstructorDatum { constructor: 0, fields } => {
				let stake_ownership =
					fields.first().and_then(Self::decode_ada_based_staking_datum)?;
				let sidechain_pub_key = fields
					.get(1)
					.and_then(|d| d.as_bytestring())
					.map(|bytes| SidechainPublicKey(bytes.clone()))?;
				let sidechain_signature = fields
					.get(2)
					.and_then(|d| d.as_bytestring())
					.map(|bytes| SidechainSignature(bytes.clone()))?;
				let consumed_input = fields.get(3).and_then(Self::decode_utxo_id_datum)?;
				let _own_pkh = fields.get(4).and_then(|d| d.as_bytestring())?;
				let aura_pub_key = fields
					.get(5)
					.and_then(|d| d.as_bytestring())
					.map(|bytes| AuraPublicKey(bytes.clone()))?;
				let grandpa_pub_key = fields
					.get(6)
					.and_then(|d| d.as_bytestring())
					.map(|bytes| GrandpaPublicKey(bytes.clone()))?;
				Some(RegisterValidatorDatum {
					stake_ownership,
					sidechain_pub_key,
					sidechain_signature,
					consumed_input,
					aura_pub_key,
					grandpa_pub_key,
				})
			},

			_ => None,
		}
	}

	fn decode_ada_based_staking_datum(datum: &Datum) -> Option<AdaBasedStaking> {
		match datum {
			ConstructorDatum { constructor: 0, fields } => {
				match fields.first().zip(fields.get(1)) {
					Some((ByteStringDatum(f0), ByteStringDatum(f1))) => {
						let pub_key = TryFrom::try_from(f0.clone()).ok()?;
						Some(AdaBasedStaking { pub_key, signature: MainchainSignature(f1.clone()) })
					},
					_ => None,
				}
			},
			_ => None,
		}
	}

	fn decode_utxo_id_datum(datum: &Datum) -> Option<UtxoId> {
		match datum {
			ConstructorDatum { constructor: 0, fields } => {
				match fields.first().zip(fields.get(1)) {
					Some((f0, IntegerDatum(f1))) => {
						let tx_hash = Self::decode_tx_hash_datum(f0)?;
						let index: u16 = TryFrom::try_from(f1.clone()).ok()?;
						Some(UtxoId { tx_hash, index: UtxoIndex(index) })
					},
					_ => None,
				}
			},
			_ => None,
		}
	}

	/// Plutus type for TxHash is a sum type, we can parse only variant with constructor 0.
	fn decode_tx_hash_datum(datum: &Datum) -> Option<McTxHash> {
		match datum {
			ConstructorDatum { constructor: 0, fields } => {
				let bytes = fields.first().and_then(|d| d.as_bytestring())?;
				Some(McTxHash(TryFrom::try_from(bytes.clone()).ok()?))
			},
			_ => None,
		}
	}

	fn get_epoch_of_data_storage(
		&self,
		epoch_of_data_usage: McEpochNumber,
	) -> Result<McEpochNumber> {
		if epoch_of_data_usage.0 < 2 {
			Err(BadRequest(format!(
				"Minimum supported epoch of data usage is 2, but {} was provided",
				epoch_of_data_usage.0
			)))
		} else {
			Ok(McEpochNumber(epoch_of_data_usage.0 - 2))
		}
	}
}
