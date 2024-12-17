use anyhow::anyhow;
use ogmios_client::query_ledger_state::QueryUtxoByUtxoId;
use sidechain_domain::UtxoId;
use std::time::Duration;
use tokio_retry::{strategy::FixedInterval, Retry};
// test ci
pub trait AwaitTx {
	#[allow(async_fn_in_trait)]
	async fn await_tx_output<Q: QueryUtxoByUtxoId>(
		&self,
		query: &Q,
		utxo_id: UtxoId,
	) -> anyhow::Result<()>;
}

pub struct FixedDelayRetries {
	delay: Duration,
	retries: usize,
}

impl FixedDelayRetries {
	pub fn new(delay: Duration, retries: usize) -> Self {
		Self { delay, retries }
	}

	pub fn two_minutes() -> Self {
		Self { delay: Duration::from_secs(5), retries: 23 }
	}
}

impl AwaitTx for FixedDelayRetries {
	async fn await_tx_output<Q: QueryUtxoByUtxoId>(
		&self,
		query: &Q,
		utxo_id: UtxoId,
	) -> anyhow::Result<()> {
		let strategy = FixedInterval::new(self.delay).take(self.retries);
		let _ = Retry::spawn(strategy, || async {
			log::info!("Probing for transaction output '{}'", utxo_id);
			let utxo = query
				.query_utxo_by_id(utxo_id.tx_hash.into(), utxo_id.index.0)
				.await
				.map_err(|_| ())?;
			utxo.ok_or(())
		})
		.await
		.map_err(|_| {
			anyhow!(
				"Retries for confirmation of transaction '{}' exceeded the limit",
				hex::encode(utxo_id.tx_hash.0)
			)
		})?;
		log::info!("Transaction output '{}'", hex::encode(utxo_id.tx_hash.0));
		Ok(())
	}
}

#[cfg(test)]
pub(crate) mod mock {
	use super::AwaitTx;
	use ogmios_client::query_ledger_state::QueryUtxoByUtxoId;

	pub(crate) struct ImmediateSuccess;

	impl AwaitTx for ImmediateSuccess {
		async fn await_tx_output<Q: QueryUtxoByUtxoId>(
			&self,
			_query: &Q,
			_utxo_id: sidechain_domain::UtxoId,
		) -> anyhow::Result<()> {
			Ok(())
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{AwaitTx, FixedDelayRetries};
	use ogmios_client::{
		query_ledger_state::QueryUtxoByUtxoId,
		types::{OgmiosTx, OgmiosUtxo},
		OgmiosClientError,
	};
	use sidechain_domain::{McTxHash, UtxoId, UtxoIndex};
	use std::{cell::RefCell, time::Duration};

	#[tokio::test]
	async fn immediate_success() {
		let mock =
			MockQueryUtxoByUtxoId { responses: RefCell::new(vec![Ok(Some(awaited_utxo()))]) };
		FixedDelayRetries::new(Duration::from_millis(1), 3)
			.await_tx_output(&mock, awaited_utxo_id())
			.await
			.unwrap();
	}

	#[tokio::test]
	async fn success_in_2nd_attempt() {
		let mock = MockQueryUtxoByUtxoId {
			responses: RefCell::new(vec![Ok(None), Ok(Some(awaited_utxo()))]),
		};
		FixedDelayRetries::new(Duration::from_millis(1), 3)
			.await_tx_output(&mock, awaited_utxo_id())
			.await
			.unwrap();
	}

	#[tokio::test]
	async fn all_attempts_result_not_found() {
		let mock =
			MockQueryUtxoByUtxoId { responses: RefCell::new(vec![Ok(None), Ok(None), Ok(None)]) };
		let result = FixedDelayRetries::new(Duration::from_millis(1), 2)
			.await_tx_output(&mock, awaited_utxo_id())
			.await;
		assert!(result.is_err())
	}

	#[tokio::test]
	async fn all_attempts_failed() {
		let mock = MockQueryUtxoByUtxoId {
			responses: RefCell::new(vec![
				Err(OgmiosClientError::RequestError("test error1".to_string())),
				Err(OgmiosClientError::RequestError("test error2".to_string())),
				Err(OgmiosClientError::RequestError("test error3".to_string())),
			]),
		};
		let result = FixedDelayRetries::new(Duration::from_millis(1), 2)
			.await_tx_output(&mock, awaited_utxo_id())
			.await;
		assert!(result.is_err())
	}

	struct MockQueryUtxoByUtxoId {
		responses: RefCell<Vec<Result<Option<OgmiosUtxo>, OgmiosClientError>>>,
	}

	impl QueryUtxoByUtxoId for MockQueryUtxoByUtxoId {
		async fn query_utxo_by_id(
			&self,
			tx: OgmiosTx,
			index: u16,
		) -> Result<Option<OgmiosUtxo>, OgmiosClientError> {
			let UtxoId { tx_hash: McTxHash(awaited_id), index: UtxoIndex(awaited_index) } =
				awaited_utxo_id();
			if tx.id == awaited_id && index == awaited_index {
				self.responses.borrow_mut().pop().unwrap()
			} else {
				Ok(None)
			}
		}
	}

	fn awaited_utxo_id() -> UtxoId {
		UtxoId { tx_hash: McTxHash([7u8; 32]), index: UtxoIndex(1) }
	}

	fn awaited_utxo() -> OgmiosUtxo {
		OgmiosUtxo { transaction: OgmiosTx { id: [7u8; 32] }, index: 1, ..Default::default() }
	}
}
