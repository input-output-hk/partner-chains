use std::collections::HashMap;

use crate::{client::*, *};
use async_trait::async_trait;
use authority_selection_inherents::*;
use blockfrost_openapi::models::{
	address_utxo_content_inner::AddressUtxoContentInner, block_content::BlockContent,
};
use futures::StreamExt;
use itertools::Itertools;
use partner_chains_plutus_data::registered_candidates::RegisterValidatorDatum;
use sidechain_domain::*;

pub struct AuthoritySelectionDataSourceImpl {
	client: MiniBFClient,
}

impl AuthoritySelectionDataSourceImpl {
	pub fn new(client: MiniBFClient) -> Self {
		Self { client }
	}
}

#[async_trait]
impl AuthoritySelectionDataSource for AuthoritySelectionDataSourceImpl {
	async fn get_ariadne_parameters(
		&self,
		_epoch_number: McEpochNumber,
		_d_parameter_validator: PolicyId,
		_permissioned_candidates_validator: PolicyId,
	) -> Result<AriadneParameters> {
		Err("not implemented".into())
	}

	async fn get_candidates(
		&self,
		epoch_number: McEpochNumber,
		committee_candidate_address: MainchainAddress,
	) -> Result<Vec<CandidateRegistrations>> {
		let epoch = self.get_epoch_of_data_storage(epoch_number)?;
		let candidates = self.get_registered_candidates(epoch, committee_candidate_address).await?;
		let pools = self.client.pools_extended().await?;
		let stake_map: HashMap<MainchainKeyHash, StakeDelegation> = futures::stream::iter(pools)
			.filter_map(|pool| async move {
				let history = self.client.pools_history(&pool.pool_id).await.expect("TODO");
				match history.into_iter().find(|h| h.epoch == epoch.0 as i32) {
					Some(e) => Some((
						MainchainKeyHash(pool.pool_id.as_bytes().try_into().expect("TODO")), // TODO is pool_id a pool hash?
						StakeDelegation(e.active_stake.parse::<u64>().expect("TODO")),
					)),
					None => None,
				}
			})
			.collect()
			.await;
		Ok(candidates
			.into_iter()
			.into_group_map_by(|c| c.stake_pool_pub_key.clone())
			.into_iter()
			.map(|(mainchain_pub_key, candidate_registrations)| CandidateRegistrations {
				stake_pool_public_key: mainchain_pub_key.clone(),
				registrations: candidate_registrations
					.into_iter()
					.map(Self::make_registration_data)
					.collect(),
				stake_delegation: Self::get_stake_delegation(&stake_map, &mainchain_pub_key),
			})
			.collect())
	}

	async fn get_epoch_nonce(&self, epoch_number: McEpochNumber) -> Result<Option<EpochNonce>> {
		let epoch = self.get_epoch_of_data_storage(epoch_number)?;
		let nonce: String = self.client.epochs_parameters(epoch).await?.nonce;
		Ok(Some(EpochNonce(nonce.into())))
	}

	async fn data_epoch(&self, for_epoch: McEpochNumber) -> Result<McEpochNumber> {
		self.get_epoch_of_data_storage(for_epoch)
	}
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
	keys: CandidateKeys,
	utxo_info: UtxoInfo,
}

#[derive(Clone, Debug)]
struct ParsedCandidate {
	utxo_info: UtxoInfo,
	datum: RegisterValidatorDatum,
	tx_inputs: Vec<UtxoId>,
}

impl AuthoritySelectionDataSourceImpl {
	fn make_registration_data(c: RegisteredCandidate) -> RegistrationData {
		RegistrationData {
			registration_utxo: c.registration_utxo,
			sidechain_signature: c.sidechain_signature,
			mainchain_signature: c.mainchain_signature,
			cross_chain_signature: c.cross_chain_signature,
			sidechain_pub_key: c.sidechain_pub_key,
			cross_chain_pub_key: c.cross_chain_pub_key,
			keys: c.keys,
			utxo_info: c.utxo_info,
			tx_inputs: c.tx_inputs,
		}
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
	async fn convert_utxos_to_candidates(
		&self,
		outputs: &[AddressUtxoContentInner],
	) -> Result<Vec<RegisteredCandidate>> {
		Self::parse_candidates(&self.client, outputs)
			.await
			.into_iter()
			.map(|c| {
				match c.datum {
					RegisterValidatorDatum::V0 {
						stake_ownership,
						sidechain_pub_key,
						sidechain_signature,
						registration_utxo,
						own_pkh: _own_pkh,
						aura_pub_key,
						grandpa_pub_key,
					} => Ok(RegisteredCandidate {
						stake_pool_pub_key: stake_ownership.pub_key,
						mainchain_signature: stake_ownership.signature,
						// For now we use the same key for both cross chain and sidechain actions
						cross_chain_pub_key: CrossChainPublicKey(sidechain_pub_key.0.clone()),
						cross_chain_signature: CrossChainSignature(sidechain_signature.0.clone()),
						sidechain_signature,
						sidechain_pub_key,
						keys: CandidateKeys(vec![aura_pub_key.into(), grandpa_pub_key.into()]),
						registration_utxo,
						tx_inputs: c.tx_inputs,
						utxo_info: c.utxo_info,
					}),
					RegisterValidatorDatum::V1 {
						stake_ownership,
						sidechain_pub_key,
						sidechain_signature,
						registration_utxo,
						own_pkh: _own_pkh,
						keys,
					} => Ok(RegisteredCandidate {
						stake_pool_pub_key: stake_ownership.pub_key,
						mainchain_signature: stake_ownership.signature,
						// For now we use the same key for both cross chain and sidechain actions
						cross_chain_pub_key: CrossChainPublicKey(sidechain_pub_key.0.clone()),
						cross_chain_signature: CrossChainSignature(sidechain_signature.0.clone()),
						sidechain_signature,
						sidechain_pub_key,
						keys,
						registration_utxo,
						tx_inputs: c.tx_inputs,
						utxo_info: c.utxo_info,
					}),
				}
			})
			.collect()
	}

	async fn parse_candidate(
		client: &impl MiniBFApi,
		output: &AddressUtxoContentInner,
	) -> std::result::Result<ParsedCandidate, String> {
		let datum_str = output.inline_datum.clone().ok_or(format!(
			"Missing registration datum for {:?}:{:?}",
			output.tx_hash,
			output.clone().output_index
		))?;
		let datum = cardano_serialization_lib::PlutusData::from_hex(&datum_str)
			.map_err(|e| e.to_string())?;
		let utxo_id = UtxoId {
			tx_hash: output.tx_hash.as_bytes().try_into().expect("TODO"),
			index: UtxoIndex(output.tx_index.try_into().expect("TODO")),
		};
		let register_validator_datum = RegisterValidatorDatum::try_from(datum)
			.map_err(|_| format!("Invalid registration datum for {:?}", utxo_id))?;
		let block = client.blocks_by_id(output.block.clone()).await?;
		let block_txs = client.blocks_txs(output.block.clone()).await?;
		let tx_index_within_block = block_txs
			.into_iter()
			.position(|tx_hash| tx_hash == output.tx_hash)
			.map(|pos| McTxIndexInBlock(pos as u32))
			.expect("TODO");
		let utxos = client.transactions_utxos(utxo_id.tx_hash).await?;
		let tx_inputs = utxos
			.inputs
			.iter()
			.map(|input| UtxoId {
				tx_hash: input.tx_hash.as_bytes().try_into().expect("TODO"),
				index: UtxoIndex(input.output_index.try_into().expect("TODO")),
			})
			.collect();
		Ok(ParsedCandidate {
			utxo_info: UtxoInfo {
				utxo_id,
				epoch_number: McEpochNumber(block.epoch.expect("TODO") as u32),
				block_number: McBlockNumber(block.height.expect("TODO") as u32),
				slot_number: McSlotNumber(block.slot.expect("TODO") as u64),
				tx_index_within_block,
			},
			datum: register_validator_datum,
			tx_inputs,
		})
	}

	async fn parse_candidates(
		client: &impl MiniBFApi,
		outputs: &[AddressUtxoContentInner],
	) -> Vec<ParsedCandidate> {
		let results = futures::stream::iter(outputs)
			.then(|output| async { Self::parse_candidate(client, output).await })
			.collect::<Vec<_>>()
			.await;
		results
			.into_iter()
			.filter_map(|r| match r {
				Ok(candidate) => Some(candidate.clone()),
				Err(msg) => {
					log::error!("{msg}");
					None
				},
			})
			.collect()
	}

	fn get_epoch_of_data_storage(
		&self,
		epoch_of_data_usage: McEpochNumber,
	) -> Result<McEpochNumber> {
		offset_data_epoch(&epoch_of_data_usage).map_err(|offset| {
			DataSourceError::BadRequest(format!(
				"Minimum supported epoch of data usage is {offset}, but {} was provided",
				epoch_of_data_usage.0
			))
			.into()
		})
	}

	/// Registrations state up to this block are considered as "active", after it - as "pending".
	async fn get_last_block_for_epoch(
		&self,
		epoch_number: McEpochNumber,
	) -> Result<Option<BlockContent>> {
		let block_option = self.client.epochs_blocks(epoch_number).await?.first().cloned(); // db_model::get_latest_block_for_epoch(&self.pool, epoch).await?;
		let block = match block_option {
			Some(block) => Some(self.client.blocks_by_id(block).await?),
			None => None,
		};
		Ok(block)
	}

	async fn get_registered_candidates(
		&self,
		epoch: McEpochNumber,
		committee_candidate_address: MainchainAddress,
	) -> Result<Vec<RegisteredCandidate>> {
		let registrations_block_for_epoch_opt = self.get_last_block_for_epoch(epoch).await?;
		let utxos = self.client.addresses_utxos(committee_candidate_address).await?;
		let active_utxos = match registrations_block_for_epoch_opt {
			Some(registrations_block_for_epoch) => {
				futures::stream::iter(utxos)
					.filter_map(|utxo: AddressUtxoContentInner| async move {
						if self
							.client
							.blocks_by_id(utxo.block.clone())
							.await
							.expect("failed to resolve block")
							.height
							.expect("block height missing") as u32
							> registrations_block_for_epoch.height.expect("block height missing")
								as u32
						{
							Some(utxo)
						} else {
							None
						}
					})
					.collect::<Vec<_>>()
					.await
			},
			None => vec![],
		};
		self.convert_utxos_to_candidates(&active_utxos).await
	}
}
