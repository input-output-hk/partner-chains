//!
//! Specification for deposit transaction:
//!
//! Consumes:
//! - UTXO at the Reserve Validator address
//!
//! Outputs:
//! - UTXO at the illiquid supply validator address with all the Reserve Tokens, plutus data Constr 0 []
//! - UTXO at the payment address with change and governance token
//!
//! Mints:
//! - Governance Token
//! - Reserve Auth Policy Token token -1 (burn)
//!
//! Reference UTOXs:
//! - Version Oracle Validator script
//! - Reserve Auth Policy script
//! - Reserve Validator script
//! - Illiquid Supply Validator script

use crate::await_tx::AwaitTx;
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use sidechain_domain::{McTxHash, UtxoId};

/// Spends current UTXO at validator address to illiquid supply validator and burn reserve auth policy token, preventing further operations.
pub async fn handover_reserve<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	payment_key: [u8; 32],
	client: &T,
	await_tx: &A,
) -> anyhow::Result<McTxHash> {
	todo!("Implement handover_reserve")
}
