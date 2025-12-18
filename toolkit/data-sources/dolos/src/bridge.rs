use crate::{
	Result,
	client::{
		MiniBFClient, api::MiniBFApi, conversions::metadata_from_response, minibf::format_asset_id,
	},
};
use blockfrost_openapi::models::{
	tx_content::TxContent, tx_content_metadata_inner::TxContentMetadataInner,
	tx_content_output_amount_inner::TxContentOutputAmountInner, tx_content_utxo::TxContentUtxo,
};
use partner_chains_plutus_data::{
	VersionedMetadatum,
	bridge::{TokenTransferMetadatum, TokenTransferMetadatumV1},
};
use sidechain_domain::*;
use sp_partner_chains_bridge::{
	BridgeDataCheckpoint, BridgeTransferV1, MainChainScripts, TokenBridgeDataSource,
};
use std::fmt::Debug;
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
where
	RecipientAddress: Debug,
	RecipientAddress: (for<'a> TryFrom<&'a [u8]>),
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
			BridgeDataCheckpoint::Tx(tx_hash) => {
				let TxBlockInfo { block_number, tx_ix } =
					get_block_info_for_utxo(&self.client, tx_hash).await?.ok_or(format!(
						"Could not find block info for data checkpoint: {data_checkpoint:?}"
					))?;
				ResolvedBridgeDataCheckpoint::Tx { block_number, tx_ix }
			},
			BridgeDataCheckpoint::Block(number) => {
				ResolvedBridgeDataCheckpoint::Block { number: number.into() }
			},
		};

		let asset = AssetId {
			policy_id: main_chain_scripts.token_policy_id.into(),
			asset_name: main_chain_scripts.token_asset_name.into(),
		};
		let current_mc_block_height: McBlockNumber = McBlockNumber(
			current_mc_block.height.expect("current mc block has valid height") as u32,
		);
		let utxos = get_bridge_utxos_tx(
			&self.client,
			&main_chain_scripts.illiquid_circulation_supply_validator_address.into(),
			asset,
			data_checkpoint,
			current_mc_block_height,
			Some(max_transfers),
		)
		.await?;

		let new_checkpoint = match utxos.last() {
			None => BridgeDataCheckpoint::Block(current_mc_block_height),
			Some(_) if (utxos.len() as u32) < max_transfers => {
				BridgeDataCheckpoint::Block(current_mc_block_height)
			},
			Some(utxo) => BridgeDataCheckpoint::Tx(utxo.tx_hash),
		};

		let transfers = utxos.into_iter().flat_map(utxo_to_transfer).collect();

		Ok((transfers, new_checkpoint))
	}
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
		return Some(BridgeTransferV1::InvalidTransfer { token_amount, tx_hash: utxo.tx_hash });
	};

	let transfer = match TokenTransferMetadatum::decode(datum) {
		Ok(TokenTransferMetadatum::V1(TokenTransferMetadatumV1::UserTransfer { receiver })) => {
			match RecipientAddress::try_from(receiver.0.as_ref()) {
				Ok(recipient) => BridgeTransferV1::UserTransfer { token_amount, recipient },
				Err(_) => BridgeTransferV1::InvalidTransfer { token_amount, tx_hash: utxo.tx_hash },
			}
		},
		Ok(TokenTransferMetadatum::V1(TokenTransferMetadatumV1::ReserveTransfer)) => {
			BridgeTransferV1::ReserveTransfer { token_amount }
		},
		Err(_) => BridgeTransferV1::InvalidTransfer { token_amount, tx_hash: utxo.tx_hash },
	};

	Some(transfer)
}

pub(crate) struct BridgeUtxo {
	pub(crate) block_number: McBlockNumber,
	pub(crate) tx_ix: McTxIndexInBlock,
	pub(crate) tx_hash: McTxHash,
	pub(crate) tokens_out: NativeTokenAmount,
	pub(crate) tokens_in: NativeTokenAmount,
	pub(crate) datum: Option<cardano_serialization_lib::TransactionMetadatum>,
}

impl BridgeUtxo {
	pub(crate) fn ordering_key(&self) -> UtxoOrderingKey {
		(self.block_number, self.tx_ix)
	}
}

pub(crate) type UtxoOrderingKey = (McBlockNumber, McTxIndexInBlock);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TxBlockInfo {
	pub(crate) block_number: McBlockNumber,
	pub(crate) tx_ix: McTxIndexInBlock,
}

pub(crate) async fn get_block_info_for_utxo(
	client: &MiniBFClient,
	tx_hash: McTxHash,
) -> Result<Option<TxBlockInfo>> {
	let tx = client.transaction_by_hash(tx_hash).await?;
	Ok(Some(TxBlockInfo {
		block_number: McBlockNumber(tx.block_height as u32),
		tx_ix: McTxIndexInBlock(tx.index as u32),
	}))
}

#[derive(Clone)]
pub(crate) enum ResolvedBridgeDataCheckpoint {
	Tx { block_number: McBlockNumber, tx_ix: McTxIndexInBlock },
	Block { number: McBlockNumber },
}

impl ResolvedBridgeDataCheckpoint {
	fn block_number(&self) -> McBlockNumber {
		match self {
			ResolvedBridgeDataCheckpoint::Tx { block_number, .. } => *block_number,
			ResolvedBridgeDataCheckpoint::Block { number } => *number,
		}
	}
}

pub(crate) async fn get_bridge_utxos_tx(
	client: &MiniBFClient,
	icp_address: &MainchainAddress,
	native_token: AssetId,
	checkpoint: ResolvedBridgeDataCheckpoint,
	to_block: McBlockNumber,
	max_utxos: Option<u32>,
) -> Result<Vec<BridgeUtxo>> {
	let txs = client.assets_transactions(native_token.clone()).await?;
	let checkpoint_block_no = checkpoint.block_number().0;
	let futures = txs.into_iter().map(|a| async move {
		let block_no = a.block_height as u32;
		if checkpoint_block_no < block_no && block_no <= to_block.0 {
			let tx_hash = McTxHash::from_hex_unsafe(&a.tx_hash);
			let utxos = client.transactions_utxos(tx_hash).await?;
			let tx = client.transaction_by_hash(tx_hash).await?;
			let tx_metadata =
				client.transaction_metadata(&McTxHash::from_hex_unsafe(&tx.hash)).await?;
			Result::Ok(Some((utxos, tx_metadata, tx)))
		} else {
			Result::Ok(None)
		}
	});
	let mut bridge_utxos = futures::future::try_join_all(futures)
		.await?
		.into_iter()
		.flatten()
		.filter(|(_, _, tx)| match checkpoint {
			ResolvedBridgeDataCheckpoint::Tx { block_number, tx_ix }
				if (tx.block_height, tx.index) <= (block_number.0 as i32, tx_ix.0 as i32) =>
			{
				false
			},
			ResolvedBridgeDataCheckpoint::Block { number }
				if tx.block_height <= number.0 as i32 =>
			{
				false
			},
			_ => true,
		})
		.flat_map(|(utxos, tx_metadata, tx): (TxContentUtxo, TxContentMetadataInner, TxContent)| {
			let native_token = native_token.clone();
			let input_tokens = utxos
				.inputs
				.iter()
				.filter(|i| i.address == icp_address.to_string())
				.map(|input| get_all_tokens(&input.amount, &native_token))
				.sum();

			let output_tokens = utxos
				.outputs
				.iter()
				.filter(|o| o.address == icp_address.to_string())
				.map(|input| get_all_tokens(&input.amount, &native_token))
				.sum();

			Some(BridgeUtxo {
				block_number: McBlockNumber(tx.block_height as u32),
				tokens_out: NativeTokenAmount(output_tokens),
				tokens_in: NativeTokenAmount(input_tokens),
				datum: Some(metadata_from_response(tx_metadata).ok()?),
				tx_ix: McTxIndexInBlock(tx.index as u32),
				tx_hash: McTxHash::from_hex_unsafe(&tx.hash),
			})
		})
		.collect::<Vec<_>>();
	bridge_utxos.sort_by_key(|b| b.ordering_key());

	if let Some(max_utxos) = max_utxos {
		bridge_utxos.truncate(max_utxos as usize);
	}

	Ok(bridge_utxos)
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
