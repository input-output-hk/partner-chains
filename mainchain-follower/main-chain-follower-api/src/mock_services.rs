use crate::*;
use async_trait::async_trait;

#[cfg(feature = "block-source")]
pub use block::*;
#[cfg(feature = "block-source")]
mod block {
	use super::*;
	use crate::block::*;
	use crate::common::*;
	use derive_new::new;

	#[derive(new, Clone)]
	pub struct MockBlockDataSource {
		pub mainchain_block: MainchainBlock,
		pub stable_blocks: Vec<MainchainBlock>,
	}

	impl Default for MockBlockDataSource {
		fn default() -> Self {
			Self { mainchain_block: mock_block(), stable_blocks: vec![mock_stable_block()] }
		}
	}
	impl MockBlockDataSource {
		pub fn with_mainchain_block(self, block: MainchainBlock) -> Self {
			Self { mainchain_block: block, ..self }
		}

		pub fn with_stable_blocks(self, blocks: Vec<MainchainBlock>) -> Self {
			Self { stable_blocks: blocks, ..self }
		}

		pub fn with_one_stable_block(self, block: MainchainBlock) -> Self {
			Self { stable_blocks: vec![block], ..self }
		}

		pub fn push_stable_block(&mut self, block: MainchainBlock) {
			self.stable_blocks.push(block);
		}

		pub fn get_all_stable_blocks(&self) -> Vec<MainchainBlock> {
			self.stable_blocks.clone()
		}
	}

	#[async_trait]
	impl BlockDataSource for MockBlockDataSource {
		async fn get_latest_block_info(&self) -> Result<MainchainBlock> {
			Ok(self.mainchain_block.clone())
		}

		async fn get_latest_stable_block_for(
			&self,
			_reference_timestamp: Timestamp,
		) -> Result<Option<MainchainBlock>> {
			Ok(self.stable_blocks.last().cloned())
		}

		async fn get_stable_block_for(
			&self,
			hash: McBlockHash,
			_reference_timestamp: Timestamp,
		) -> Result<Option<MainchainBlock>> {
			Ok(self.stable_blocks.iter().find(|b| b.hash == hash).cloned())
		}
	}

	fn mock_block() -> MainchainBlock {
		MainchainBlock {
			number: McBlockNumber(5),
			hash: McBlockHash([123; 32]),
			epoch: McEpochNumber(3),
			slot: McSlotNumber(12),
			timestamp: 8,
		}
	}

	pub fn mock_stable_block() -> MainchainBlock {
		MainchainBlock {
			number: McBlockNumber(1),
			hash: McBlockHash([1; 32]),
			epoch: McEpochNumber(2),
			slot: McSlotNumber(3),
			timestamp: 4,
		}
	}
}

#[cfg(feature = "candidate-source")]
pub use candidates::*;

#[cfg(feature = "candidate-source")]
mod candidates {
	use super::*;
	use crate::candidate::*;
	use crate::DataSourceError::ExpectedDataNotFound;

	#[derive(Clone)]
	pub struct MockCandidateDataSource {
		/// Each entry in each field is returned when queried for epoch equal to its index.
		/// For example `candidates[0]` is the list of candidates that will be returned for epoch 0.
		/// `candidates[1]` is the list of candidates that will be returned for epoch 1 and so on.
		pub candidates: Vec<Vec<CandidateRegistrations>>,
		pub permissioned_candidates: Vec<Option<Vec<RawPermissionedCandidateData>>>,
	}

	impl Default for MockCandidateDataSource {
		fn default() -> Self {
			Self {
				candidates: vec![vec![], vec![]],
				permissioned_candidates: vec![Some(vec![]), Some(vec![])],
			}
		}
	}

	impl MockCandidateDataSource {
		pub fn with_candidates_per_epoch(
			self,
			candidates: Vec<Vec<CandidateRegistrations>>,
		) -> Self {
			Self { candidates, ..self }
		}

		pub fn with_permissioned_candidates(
			self,
			permissioned_candidates: Vec<Option<Vec<RawPermissionedCandidateData>>>,
		) -> Self {
			Self { permissioned_candidates, ..self }
		}
	}

	#[async_trait]
	impl CandidateDataSource for MockCandidateDataSource {
		async fn get_ariadne_parameters(
			&self,
			epoch_number: McEpochNumber,
			_d_parameter_policy: PolicyId,
			_permissioned_candidates_policy: PolicyId,
		) -> Result<AriadneParameters> {
			match self.permissioned_candidates.get(epoch_number.0 as usize) {
				Some(Some(candidates)) => Ok(AriadneParameters {
					d_parameter: DParameter {
						num_permissioned_candidates: 3,
						num_registered_candidates: 2,
					},
					permissioned_candidates: candidates.clone(),
				}),
				_ => Err(ExpectedDataNotFound(
					"mock was called with unexpected argument".to_string(),
				)),
			}
		}

		async fn get_candidates(
			&self,
			epoch_number: McEpochNumber,
			_committee_candidate_address: MainchainAddress,
		) -> Result<Vec<CandidateRegistrations>> {
			Ok(self.candidates.get(epoch_number.0 as usize).cloned().unwrap_or(vec![]))
		}

		async fn get_epoch_nonce(&self, _epoch: McEpochNumber) -> Result<Option<EpochNonce>> {
			Ok(Some(EpochNonce(vec![42u8])))
		}

		async fn data_epoch(&self, for_epoch: McEpochNumber) -> Result<McEpochNumber> {
			Ok(for_epoch)
		}
	}
}
