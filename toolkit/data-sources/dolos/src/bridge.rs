use crate::{
	Result,
	client::{MiniBFClient, api::MiniBFApi, minibf::format_asset_id},
};
use blockfrost_openapi::models::{
	tx_content::TxContent, tx_content_output_amount_inner::TxContentOutputAmountInner,
	tx_content_utxo::TxContentUtxo,
};
use cardano_serialization_lib::PlutusData;
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
impl<RecipientAddress: Send + Sync> TokenBridgeDataSource<RecipientAddress>
	for TokenBridgeDataSourceImpl<RecipientAddress>
{
	async fn get_transfers(
		&self,
		_main_chain_scripts: MainChainScripts,
		data_checkpoint: BridgeDataCheckpoint,
		_max_transfers: u32,
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

		let asset = Asset {
			policy_id: main_chain_scripts.token_policy_id.into(),
			asset_name: main_chain_scripts.token_asset_name.into(),
		};
		let utxos = get_bridge_utxos_tx(
			&main_chain_scripts.illiquid_circulation_supply_validator_address.into(),
			asset,
			data_checkpoint,
			current_mc_block.block_no,
			Some(max_transfers),
		)
		.await?;

		let new_checkpoint = match utxos.last() {
			None => BridgeDataCheckpoint::Block(current_mc_block.block_no.into()),
			Some(_) if (utxos.len() as u32) < max_transfers => {
				BridgeDataCheckpoint::Block(current_mc_block.block_no.into())
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
}

pub(crate) enum ResolvedBridgeDataCheckpoint {
	Utxo { block_number: BlockNumber, tx_ix: TxIndexInBlock, tx_out_ix: TxIndex },
	Block { number: BlockNumber },
}

pub(crate) async fn get_bridge_utxos_tx(
	client: &MiniBFClient,
	icp_address: &MainchainAddress,
	native_token: AssetId,
	checkpoint: ResolvedBridgeDataCheckpoint,
	to_block: McBlockNumber,
	max_utxos: Option<u32>,
) -> Result<Vec<BridgeUtxo>> {
	let x = client.assets_transactions(native_token.clone()).await?;
	let y = x.into_iter().map(|a| async move {
		if a.block_height as u32 >= to_block.0 {
			let tx_hash = McTxHash::from_hex_unsafe(&a.tx_hash);
			let x = client.transactions_utxos(tx_hash).await?;
			let y = client.transaction_by_hash(tx_hash).await?;
			Result::Ok(Some((x, y)))
		} else {
			Result::Ok(None)
		}
	});
	let mut z = futures::future::try_join_all(y)
		.await?
		.iter()
		.flatten()
		.flat_map(|(x, y): &(TxContentUtxo, TxContent)| {
			let inputs = x.inputs.iter().filter(|i| i.address == icp_address.to_string());
			let outputs = x.outputs.iter().filter(|o| o.address == icp_address.to_string());
			let native_token = native_token.clone();
			outputs.map(move |output| {
				let native_token = native_token.clone();
				let output_tokens = get_all_tokens(&output.amount, &native_token.clone());
				let input_tokens = inputs
					.clone()
					.map(move |input| get_all_tokens(&input.amount, &native_token.clone()))
					.sum();

				BridgeUtxo {
					block_number: McBlockNumber(y.block_height as u32),
					tokens_out: NativeTokenAmount(output_tokens),
					tokens_in: NativeTokenAmount(input_tokens),
					datum: output
						.inline_datum
						.clone()
						.map(|d| PlutusData::from_hex(&d).expect("valid datum")),
					tx_ix: todo!(),
					tx_hash: todo!(),
					utxo_ix: todo!(),
				}
			})
		})
		.collect::<Vec<_>>();

	match checkpoint {
		ResolvedBridgeDataCheckpoint::Block { number } => {
			query_builder.push(&format!("AND block_no > {} ", number.0));
		},
		ResolvedBridgeDataCheckpoint::Utxo { block_number, tx_ix, tx_out_ix } => {
			query_builder.push(&format!(
				"AND (block_no, tx.block_index, outputs.index) > ({}, {}, {}) ",
				block_number.0, tx_ix.0, tx_out_ix.0
			));
		},
	}

	query_builder.push("ORDER BY block.block_no, tx.block_index, outputs.index ");

	if let Some(max_utxos) = max_utxos {
		query_builder.push(&format!("LIMIT {max_utxos}"));
	}

	Err("not implemented".into())
}

fn get_all_tokens(amount: &Vec<TxContentOutputAmountInner>, asset_id: &AssetId) -> u128 {
	amount
		.iter()
		.map(|v| {
			if v.unit == format_asset_id(asset_id) {
				v.quantity.parse::<u128>().expect("valid quantity is u128")
			} else {
				0u128
			}
		})
		.sum()
}

// get_bridge_utxos_tx

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
