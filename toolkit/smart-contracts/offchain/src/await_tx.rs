use anyhow::anyhow;
use ogmios_client::query_ledger_state::QueryUtxoByUtxoId;
use sidechain_domain::{McTxHash, UtxoId};
use std::time::Duration;
use tokio_retry::{Retry, strategy::FixedInterval};

/// Trait for different strategies of waiting for a Cardano transaction to complete.
pub trait AwaitTx {
	#[allow(async_fn_in_trait)]
	/// This is used for waiting until the output of a submitted transaction can be observed.
	async fn await_tx_output<C: QueryUtxoByUtxoId>(
		&self,
		client: &C,
		tx_hash: McTxHash,
	) -> anyhow::Result<()>;
}

/// Transaction awaiting strategy that uses fixed number of retries and a fixed delay.
pub struct FixedDelayRetries {
	delay: Duration,
	retries: usize,
}

impl FixedDelayRetries {
	/// Constructs [FixedDelayRetries] with `delay` [Duration] and `retries` number of maximum retries.
	pub fn new(delay: Duration, retries: usize) -> Self {
		Self { delay, retries }
	}

	/// Constructs [FixedDelayRetries] that keeps retrying every 5 seconds for 5 minutes.
	pub fn five_minutes() -> Self {
		Self { delay: Duration::from_secs(5), retries: 59 }
	}
}

impl AwaitTx for FixedDelayRetries {
	async fn await_tx_output<C: QueryUtxoByUtxoId>(
		&self,
		client: &C,
		tx_hash: McTxHash,
	) -> anyhow::Result<()> {
		let strategy = FixedInterval::new(self.delay).take(self.retries);
		let utxo_id = UtxoId::new(tx_hash.0, 0);
		let _ = Retry::spawn(strategy, || async {
			log::info!("Probing for transaction output '{}'", utxo_id);
			let utxo = client.query_utxo_by_id(utxo_id).await.map_err(|_| ())?;
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
			_utxo_id: sidechain_domain::McTxHash,
		) -> anyhow::Result<()> {
			Ok(())
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{AwaitTx, FixedDelayRetries};
	use ogmios_client::{
		OgmiosClientError,
		query_ledger_state::QueryUtxoByUtxoId,
		types::{OgmiosTx, OgmiosUtxo},
	};
	use sidechain_domain::McTxHash;
	use std::{cell::RefCell, time::Duration};

	#[tokio::test]
	async fn immediate_success() {
		let mock =
			MockQueryUtxoByUtxoId { responses: RefCell::new(vec![Ok(Some(awaited_utxo()))]) };
		FixedDelayRetries::new(Duration::from_millis(1), 3)
			.await_tx_output(&mock, awaited_tx_hash())
			.await
			.unwrap();
	}

	#[tokio::test]
	async fn success_in_2nd_attempt() {
		let mock = MockQueryUtxoByUtxoId {
			responses: RefCell::new(vec![Ok(None), Ok(Some(awaited_utxo()))]),
		};
		FixedDelayRetries::new(Duration::from_millis(1), 3)
			.await_tx_output(&mock, awaited_tx_hash())
			.await
			.unwrap();
	}

	#[tokio::test]
	async fn all_attempts_result_not_found() {
		let mock =
			MockQueryUtxoByUtxoId { responses: RefCell::new(vec![Ok(None), Ok(None), Ok(None)]) };
		let result = FixedDelayRetries::new(Duration::from_millis(1), 2)
			.await_tx_output(&mock, awaited_tx_hash())
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
			.await_tx_output(&mock, awaited_tx_hash())
			.await;
		assert!(result.is_err())
	}

	struct MockQueryUtxoByUtxoId {
		responses: RefCell<Vec<Result<Option<OgmiosUtxo>, OgmiosClientError>>>,
	}

	impl QueryUtxoByUtxoId for MockQueryUtxoByUtxoId {
		async fn query_utxo_by_id(
			&self,
			utxo: sidechain_domain::UtxoId,
		) -> Result<Option<OgmiosUtxo>, OgmiosClientError> {
			if utxo.tx_hash == awaited_tx_hash() {
				self.responses.borrow_mut().pop().unwrap()
			} else {
				Ok(None)
			}
		}
	}

	fn awaited_tx_hash() -> McTxHash {
		McTxHash([7u8; 32])
	}

	fn awaited_utxo() -> OgmiosUtxo {
		OgmiosUtxo { transaction: OgmiosTx { id: [7u8; 32] }, index: 1, ..Default::default() }
	}
}
