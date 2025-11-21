use crate::{client::{MiniBFClient, api::MiniBFApi}, Result, DataSourceError};
use async_trait::async_trait;
use cardano_serialization_lib::PlutusData;
use partner_chains_plutus_data::governed_map::GovernedMapDatum;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::*;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};
use std::collections::BTreeMap;

pub struct GovernedMapDataSourceImpl {
	client: MiniBFClient,
}

impl GovernedMapDataSourceImpl {
	pub fn new(client: MiniBFClient) -> Self {
		Self { client }
	}
}

#[async_trait]
impl GovernedMapDataSource for GovernedMapDataSourceImpl {
	async fn get_state_at_block(
		&self,
		mc_block: McBlockHash,
		main_chain_scripts: MainChainScriptsV1,
	) -> Result<BTreeMap<String, ByteString>> {
		// Get the block to ensure it exists and get its number
		let block = self.client.blocks_by_id(mc_block.clone()).await?;
		let block_number = McBlockNumber(block.height.unwrap_or_default().try_into().unwrap_or(0u32));

		// Get all UTXOs at the governed map validator address
		let utxos = self.client.addresses_utxos(main_chain_scripts.validator_address.clone()).await?;

		// Filter UTXOs that:
		// 1. Contain the governed map asset
		// 2. Were created before or at the target block
		let asset_unit = format_asset_unit(&main_chain_scripts.asset_policy_id);
		let mut mappings = BTreeMap::new();

		for utxo in utxos {
			// Check if this UTXO was created before or at target block
			let tx_hash = McTxHash::from_hex_unsafe(&utxo.tx_hash);
			let tx = self.client.transaction_by_hash(tx_hash).await?;
			let utxo_block_height = tx.block_height as u32;

			if utxo_block_height > block_number.0 {
				continue;
			}

			// Check if UTXO contains the governed map asset
			let has_asset = utxo.amount.iter().any(|a| a.unit == asset_unit);
			if !has_asset {
				continue;
			}

			// Parse the datum
			if let Some(datum_hex) = &utxo.inline_datum {
				match PlutusData::from_hex(datum_hex) {
					Ok(plutus_data) => {
						match GovernedMapDatum::try_from(plutus_data) {
							Ok(GovernedMapDatum { key, value }) => {
								mappings.insert(key, value);
							},
							Err(err) => {
								log::warn!("Failed to parse GovernedMapDatum: {}", err);
							},
						}
					},
					Err(err) => {
						log::warn!("Failed to parse PlutusData from hex: {}", err);
					},
				}
			}
		}

		Ok(mappings)
	}

	async fn get_mapping_changes(
		&self,
		since_mc_block: Option<McBlockHash>,
		up_to_mc_block: McBlockHash,
		scripts: MainChainScriptsV1,
	) -> Result<Vec<(String, Option<ByteString>)>> {
		// Get current state at up_to_mc_block
		let current_mappings = self.get_state_at_block(up_to_mc_block, scripts.clone()).await?;

		// If no since_mc_block, return all current mappings as additions
		let Some(since_mc_block) = since_mc_block else {
			let changes = current_mappings
				.into_iter()
				.map(|(key, value)| (key, Some(value)))
				.collect();
			return Ok(changes);
		};

		// Get previous state at since_mc_block
		let previous_mappings = self.get_state_at_block(since_mc_block, scripts).await?;

		// Calculate changes
		let mut changes = Vec::new();

		// Find additions and modifications
		for (key, value) in current_mappings.iter() {
			if previous_mappings.get(key) != Some(value) {
				changes.push((key.clone(), Some(value.clone())));
			}
		}

		// Find deletions
		for key in previous_mappings.keys() {
			if !current_mappings.contains_key(key) {
				changes.push((key.clone(), None));
			}
		}

		Ok(changes)
	}
}

fn format_asset_unit(policy_id: &PolicyId) -> String {
	// Asset unit format in blockfrost is policy_id + asset_name (hex)
	// For empty asset names, it's just the policy_id without "0x" prefix
	policy_id.to_hex_string()[2..].to_string()
}
