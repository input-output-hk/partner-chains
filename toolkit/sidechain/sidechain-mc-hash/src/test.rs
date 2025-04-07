mod inherent_digest_tests {
	use crate::mock::*;
	use crate::*;
	use sp_partner_chains_consensus_aura::inherent_digest::InherentDigest;

	#[tokio::test]
	async fn from_inherent_data_works() {
		let inherent_data = MockMcHashInherentDataProvider { mc_hash: McBlockHash([42; 32]) }
			.create_inherent_data()
			.await
			.unwrap();

		let result = McHashInherentDigest::from_inherent_data(&inherent_data)
			.expect("from_inherent_data should not fail");

		assert_eq!(result, vec![DigestItem::PreRuntime(MC_HASH_DIGEST_ID, vec![42; 32])])
	}

	#[tokio::test]
	async fn value_from_digest_works() {
		let digest_to_ignore = DigestItem::PreRuntime(*b"irlv", vec![0; 32]);
		let digest = DigestItem::PreRuntime(MC_HASH_DIGEST_ID, vec![42; 32]);

		let result = McHashInherentDigest::value_from_digest(&[digest_to_ignore, digest])
			.expect("value_from_digest should not fail");

		assert_eq!(result, McBlockHash([42; 32]))
	}
}

mod validation_tests {
	use crate::mock::MockMcHashDataSource;
	use crate::McHashInherentError::*;
	use crate::*;
	use sp_consensus_slots::Slot;
	use sp_consensus_slots::SlotDuration;
	use sp_runtime::testing::Digest;
	use sp_runtime::testing::Header;
	use sp_runtime::traits::Header as HeaderT;

	#[tokio::test]
	async fn mc_state_reference_block_numbers_should_not_decrease() {
		let mc_block_hash = McBlockHash([2; 32]);
		let parent_stable_block_hash = McBlockHash([1; 32]);
		let slot_duration = SlotDuration::from_millis(1000);

		let parent_stable_block = MainchainBlock {
			number: McBlockNumber(1),
			hash: parent_stable_block_hash.clone(),
			epoch: McEpochNumber(2),
			slot: McSlotNumber(3),
			timestamp: 4,
		};

		let next_stable_block = MainchainBlock {
			number: McBlockNumber(parent_stable_block.number.0 - 1),
			hash: mc_block_hash.clone(),
			slot: McSlotNumber(parent_stable_block.slot.0 - 1),
			timestamp: parent_stable_block.timestamp - 1,
			epoch: McEpochNumber(parent_stable_block.epoch.0),
		};
		let mc_hash_data_source =
			MockMcHashDataSource::new(vec![parent_stable_block, next_stable_block], vec![]);

		let err = McHashInherentDataProvider::new_verification(
			mock_header(parent_stable_block_hash),
			Some(Slot::from(1)),
			30.into(),
			mc_block_hash.clone(),
			slot_duration,
			&mc_hash_data_source,
		)
		.await;
		assert!(err.is_err());
		assert_eq!(
			err.unwrap_err().to_string(),
			McStateReferenceRegressed(mc_block_hash, 30.into(), McBlockNumber(0), McBlockNumber(1))
				.to_string()
		);
	}

	#[tokio::test]
	async fn proposed_mc_state_reference_block_numbers_should_not_decrease() {
		let mc_block_hash = McBlockHash([2; 32]);
		let parent_stable_block_hash = McBlockHash([3; 32]);
		let slot_duration = SlotDuration::from_millis(1000);

		let parent_stable_block = MainchainBlock {
			number: McBlockNumber(3),
			hash: parent_stable_block_hash.clone(),
			epoch: McEpochNumber(1),
			slot: McSlotNumber(30),
			timestamp: 300,
		};

		let current_latest_stable_block_from_db_sync = MainchainBlock {
			number: McBlockNumber(2),
			hash: mc_block_hash.clone(),
			slot: McSlotNumber(20),
			epoch: McEpochNumber(1),
			timestamp: 200,
		};

		let mc_hash_data_source = MockMcHashDataSource::new(
			vec![current_latest_stable_block_from_db_sync],
			vec![parent_stable_block.clone()],
		);
		let provider = McHashInherentDataProvider::new_proposal(
			mock_header(parent_stable_block_hash),
			&mc_hash_data_source,
			Slot::from(1),
			slot_duration,
		)
		.await
		.unwrap();
		assert_eq!(provider.mc_block, parent_stable_block);
	}

	#[tokio::test]
	async fn propose_fails_if_parent_mc_state_cannot_be_found() {
		let mc_block_hash = McBlockHash([2; 32]);
		let parent_stable_block_hash = McBlockHash([3; 32]);
		let slot_duration = SlotDuration::from_millis(1000);

		let current_latest_stable_block_from_db_sync = MainchainBlock {
			number: McBlockNumber(2),
			hash: mc_block_hash.clone(),
			slot: McSlotNumber(20),
			epoch: McEpochNumber(1),
			timestamp: 200,
		};

		let mc_hash_data_source =
			MockMcHashDataSource::new(vec![current_latest_stable_block_from_db_sync], vec![]);
		let err = McHashInherentDataProvider::new_proposal(
			mock_header(parent_stable_block_hash),
			&mc_hash_data_source,
			Slot::from(1),
			slot_duration,
		)
		.await
		.unwrap_err();
		assert_eq!(err.to_string(), StableBlockNotFoundByHash(mc_block_hash).to_string());
	}

	pub fn mock_header(mc_hash: McBlockHash) -> Header {
		Header::new(
			Default::default(),
			Default::default(),
			Default::default(),
			Default::default(),
			Digest { logs: McHashInherentDigest::from_mc_block_hash(mc_hash) },
		)
	}
}
