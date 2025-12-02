use crate::{
	Result,
	client::{MiniBFClient, api::MiniBFApi, minibf::format_asset_id},
};
use blockfrost_openapi::models::{
	tx_content::TxContent, tx_content_output_amount_inner::TxContentOutputAmountInner,
};
use cardano_serialization_lib::PlutusData;
use partner_chains_plutus_data::bridge::{TokenTransferDatum, TokenTransferDatumV1};
use sidechain_domain::*;
use sp_partner_chains_bridge::{
	BridgeDataCheckpoint, BridgeTransferV1, MainChainScripts, TokenBridgeDataSource,
};
use std::marker::PhantomData;
pub struct TokenBridgeDataSourceImpl<RecipientAddress> {
	client: MiniBFClient,
	_phantom: PhantomData<RecipientAddress>,
}

impl<RecipientAddress> TokenBridgeDataSourceImpl<RecipientAddress> {
	pub fn new(client: MiniBFClient) -> Self {
		Self { client, _phantom: PhantomData::default() }
	}
}

#[async_trait::async_trait]
impl<RecipientAddress: Send + Sync + std::fmt::Debug + for<'a> TryFrom<&'a [u8]>>
	TokenBridgeDataSource<RecipientAddress> for TokenBridgeDataSourceImpl<RecipientAddress>
{
	async fn get_transfers(
		&self,
		main_chain_scripts: MainChainScripts,
		data_checkpoint: BridgeDataCheckpoint,
		max_transfers: u32,
		current_mc_block_hash: McBlockHash,
	) -> Result<(Vec<BridgeTransferV1<RecipientAddress>>, BridgeDataCheckpoint)> {
		let current_mc_block = self.client.blocks_by_id(current_mc_block_hash).await?;

		let data_checkpoint = match data_checkpoint {
			BridgeDataCheckpoint::Utxo(utxo) => {
				let TxBlockInfo { block_number, tx_ix } =
					get_block_info_for_utxo(&self.client, utxo.tx_hash.into()).await?.ok_or(
						format!(
							"Could not find block info for data checkpoint: {data_checkpoint:?}"
						),
					)?;
				ResolvedBridgeDataCheckpoint::Utxo {
					block_number,
					tx_ix,
					tx_out_ix: utxo.index.into(),
				}
			},
			BridgeDataCheckpoint::Block(number) => {
				ResolvedBridgeDataCheckpoint::Block { number: number.into() }
			},
		};

		let asset = AssetId {
			policy_id: main_chain_scripts.token_policy_id.into(),
			asset_name: main_chain_scripts.token_asset_name.into(),
		};
		let to_block =
			McBlockNumber(current_mc_block.height.unwrap_or_default().try_into().unwrap_or(0u32));
		let utxos = get_bridge_utxos_tx(
			&self.client,
			&main_chain_scripts.illiquid_circulation_supply_validator_address.into(),
			asset,
			data_checkpoint,
			to_block,
			Some(max_transfers),
		)
		.await?;

		let new_checkpoint = match utxos.last() {
			None => BridgeDataCheckpoint::Block(to_block),
			Some(_) if (utxos.len() as u32) < max_transfers => {
				BridgeDataCheckpoint::Block(to_block)
			},
			Some(utxo) => BridgeDataCheckpoint::Utxo(utxo.utxo_id()),
		};

		let transfers = utxos.into_iter().flat_map(utxo_to_transfer).collect();

		Ok((transfers, new_checkpoint))
	}
}

pub(crate) struct BridgeUtxo {
	pub(crate) block_number: McBlockNumber,
	pub(crate) tx_ix: McTxIndexInBlock,
	pub(crate) tx_hash: McTxHash,
	pub(crate) utxo_ix: UtxoIndex,
	pub(crate) tokens_out: NativeTokenAmount,
	pub(crate) tokens_in: NativeTokenAmount,
	pub(crate) datum: Option<cardano_serialization_lib::PlutusData>,
}

impl BridgeUtxo {
	pub(crate) fn utxo_id(&self) -> UtxoId {
		UtxoId { tx_hash: self.tx_hash, index: self.utxo_ix }
	}
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TxBlockInfo {
	pub(crate) block_number: McBlockNumber,
	pub(crate) tx_ix: McTxIndexInBlock,
}

pub(crate) async fn get_block_info_for_utxo(
	client: &MiniBFClient,
	tx_hash: McTxHash,
) -> Result<Option<TxBlockInfo>> {
	// SELECT
	// 	block.block_no AS block_number,
	// 	tx.block_index AS tx_ix
	// FROM tx
	// JOIN block  ON block.id = tx.block_id
	// WHERE tx.hash = $tx_hash
	let tx: TxContent = client.transaction_by_hash(tx_hash).await?;
	Ok(Some(TxBlockInfo {
		block_number: McBlockNumber(tx.block_height as u32),
		tx_ix: McTxIndexInBlock(tx.index as u32),
	}))
}

pub(crate) enum ResolvedBridgeDataCheckpoint {
	Utxo { block_number: McBlockNumber, tx_ix: McTxIndexInBlock, tx_out_ix: UtxoIndex },
	Block { number: McBlockNumber },
}

pub(crate) async fn get_bridge_utxos_tx(
	client: &MiniBFClient,
	icp_address: &MainchainAddress,
	native_token: AssetId,
	checkpoint: ResolvedBridgeDataCheckpoint,
	to_block: McBlockNumber,
	max_utxos: Option<u32>,
) -> Result<Vec<BridgeUtxo>> {
	// Use the optimized endpoint to get UTXOs at ICS address filtered by the bridge token
	// This is much more efficient than querying all asset transactions
	let address_utxos =
		client.addresses_utxos_asset(icp_address.clone(), native_token.clone()).await?;

	// Process each UTXO to calculate token deltas and gather transaction info
	let futures = address_utxos.into_iter().map(|utxo| {
		let client = client.clone();
		let native_token = native_token.clone();
		let icp_address = icp_address.clone();
		async move {
			let tx_hash = match McTxHash::decode_hex(&utxo.tx_hash) {
				Ok(hash) => hash,
				Err(e) => {
					log::warn!("Failed to decode tx_hash '{}': {}", utxo.tx_hash, e);
					return Result::Ok(None);
				}
			};
			let tx = client.transaction_by_hash(tx_hash).await?;

			// Skip if beyond target block
			if (tx.block_height as u32) > to_block.0 {
				return Result::Ok(None);
			}

			// Get full transaction UTXOs to calculate input token amounts
			let tx_utxos = client.transactions_utxos(tx_hash).await?;

			// Calculate total input tokens at ICS address
			let input_tokens_total: u128 = tx_utxos
				.inputs
				.iter()
				.filter(|i| i.address == icp_address.to_string())
				.map(|input| get_all_tokens(&input.amount, &native_token))
				.sum();

			// Get output token amount from this specific UTXO
			let output_tokens = get_all_tokens(&utxo.amount, &native_token);

			let bridge_utxo = BridgeUtxo {
				block_number: McBlockNumber(tx.block_height as u32),
				tx_ix: McTxIndexInBlock(tx.index as u32),
				tx_hash,
				utxo_ix: UtxoIndex(utxo.output_index as u16),
				tokens_out: NativeTokenAmount(output_tokens),
				tokens_in: NativeTokenAmount(input_tokens_total),
				datum: utxo.inline_datum.clone().and_then(|d| match PlutusData::from_hex(&d) {
					Ok(pd) => Some(pd),
					Err(e) => {
						log::warn!("Failed to parse PlutusData from hex for tx {}: {}", tx_hash, e);
						None
					},
				}),
			};

			Result::Ok(Some(bridge_utxo))
		}
	});

	let mut utxos = futures::future::try_join_all(futures)
		.await?
		.into_iter()
		.flatten()
		.collect::<Vec<_>>();

	// Filter by checkpoint
	utxos.retain(|u| match checkpoint {
		ResolvedBridgeDataCheckpoint::Block { number } => u.block_number.0 > number.0,
		ResolvedBridgeDataCheckpoint::Utxo { block_number, tx_ix, tx_out_ix } => {
			(u.block_number.0, u.tx_ix.0, u.utxo_ix.0) > (block_number.0, tx_ix.0, tx_out_ix.0)
		},
	});

	// Sort by (block_no, tx.block_index, outputs.index)
	utxos.sort_by_key(|u| (u.block_number.0, u.tx_ix.0, u.utxo_ix.0));

	// Limit number of results
	if let Some(max) = max_utxos {
		if utxos.len() > max as usize {
			utxos.truncate(max as usize);
		}
	}

	Ok(utxos)
}

fn get_all_tokens(amount: &Vec<TxContentOutputAmountInner>, asset_id: &AssetId) -> u128 {
	amount
		.iter()
		.map(|v| {
			if v.unit == format_asset_id(asset_id) {
				match v.quantity.parse::<u128>() {
					Ok(qty) => qty,
					Err(e) => {
						log::warn!("Failed to parse token quantity '{}': {}", v.quantity, e);
						0u128
					}
				}
			} else {
				0u128
			}
		})
		.sum()
}

fn utxo_to_transfer<RecipientAddress>(
	utxo: BridgeUtxo,
) -> Option<BridgeTransferV1<RecipientAddress>>
where
	RecipientAddress: for<'a> TryFrom<&'a [u8]>,
{
	let token_delta = utxo.tokens_out.0.checked_sub(utxo.tokens_in.0)?;

	if token_delta == 0 {
		return None;
	}

	let token_amount = token_delta as u64;

	let Some(datum) = utxo.datum.clone() else {
		return Some(BridgeTransferV1::InvalidTransfer { token_amount, utxo_id: utxo.utxo_id() });
	};

	let transfer = match TokenTransferDatum::try_from(datum) {
		Ok(TokenTransferDatum::V1(TokenTransferDatumV1::UserTransfer { receiver })) => {
			match RecipientAddress::try_from(receiver.0.as_ref()) {
				Ok(recipient) => BridgeTransferV1::UserTransfer { token_amount, recipient },
				Err(_) => {
					BridgeTransferV1::InvalidTransfer { token_amount, utxo_id: utxo.utxo_id() }
				},
			}
		},
		Ok(TokenTransferDatum::V1(TokenTransferDatumV1::ReserveTransfer)) => {
			BridgeTransferV1::ReserveTransfer { token_amount }
		},
		Err(_) => BridgeTransferV1::InvalidTransfer { token_amount, utxo_id: utxo.utxo_id() },
	};

	Some(transfer)
}

/*
	* tx block no >= to_block
	* input : at icp address && token kind = assetid -> get SUM input token quantity
	* output: at icp address

  SELECT
	  block.block_no                          AS block_number
	  , tx.block_index                          AS tx_ix
	  , tx.hash                                 AS tx_hash
	  , outputs.index                           AS utxo_ix
	  , output_tokens.quantity                  AS tokens_out
	  , coalesce(sum(input_tokens.quantity), 0) AS tokens_in
	  , datum.value                             AS datum
  FROM
				tx_out      outputs
	   JOIN     tx                            ON      outputs.tx_id = tx.id
	   JOIN     block                         ON      tx.block_id = block.id
	   JOIN     ma_tx_out   output_tokens     ON      output_tokens.tx_out_id = outputs.id
	   JOIN     multi_asset                   ON      multi_asset.id = output_tokens.ident
  LEFT JOIN     datum                         ON      datum.hash = outputs.data_hash
  LEFT JOIN     tx_out     inputs             ON      inputs.consumed_by_tx_id = tx.id   AND inputs.address = $icp_address
  LEFT JOIN     ma_tx_out  input_tokens       ON      input_tokens.tx_out_id = inputs.id AND input_tokens.ident = multi_asset.id

  WHERE

	  multi_asset.policy = $native_token.policy_id AND multi_asset.name = $native_token.policy_name
  AND outputs.address = $icp_address
  AND block_no <= $to_block
*/
