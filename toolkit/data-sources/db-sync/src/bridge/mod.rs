//! Db-Sync data source used by the Partner Chain token bridge observability
//!
//! # Assumptions
//!
//! The data source implementation assumes that the utxos found at the illiquid circulating
//! supply address conform to rules that are enforced by the Partner Chains smart contracts.
//!
//! Most importantly, transactions that spend any UTXOs from the ICS can only create at most
//! one new UTXO at the ICS address. Conversely, transactions that create more than one UTXO
//! at the illiquid supply address can only spend UTXOs from outside of it. This guarantees
//! that the observability layer can always correctly identify the number of tokens transfered
//! by calculating the delta of `tokens in the new UTXO` - `tokens in the old ICS UTXOs`.

use crate::McFollowerMetrics;
use crate::db_model::*;
use crate::metrics::observed_async_trait;
use partner_chains_plutus_data::bridge::{TokenTransferDatum, TokenTransferDatumV1};
use sp_partner_chains_bridge::*;
use sqlx::PgPool;
use std::fmt::Debug;

#[cfg(test)]
mod tests;

pub(crate) mod cache;

fn utxo_to_transfer<RecipientAddress>(
	utxo: BridgeUtxo,
) -> Option<BridgeTransferV1<RecipientAddress>>
where
	RecipientAddress: for<'a> TryFrom<&'a [u8]>,
{
	let token_delta = utxo.tokens_out.checked_sub(utxo.tokens_in)?;

	if token_delta.is_zero() {
		return None;
	}

	let token_amount = token_delta.0 as u64;

	let transfer = match TokenTransferDatum::try_from(utxo.datum.0.clone()) {
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
