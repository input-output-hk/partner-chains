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
	use crate::McHashInherentError::*;
	use crate::*;
	use main_chain_follower_api::{block::MainchainBlock, mock_services::MockBlockDataSource};
	use sidechain_domain::*;
	use sp_consensus_slots::Slot;
	use sp_consensus_slots::SlotDuration;
	use sp_runtime::testing::Digest;
	use sp_runtime::testing::Header;
	use sp_runtime::traits::Header as HeaderT;

	#[tokio::test]
	async fn mc_state_reference_block_numbers_should_not_decrease() {
		let mut block_data_source = MockBlockDataSource::default();
		let parent_stable_block =
			block_data_source.get_all_stable_blocks().first().unwrap().clone();
		let mc_block_hash = McBlockHash([2; 32]);
		let slot_duration = SlotDuration::from_millis(1000);

		block_data_source.push_stable_block(MainchainBlock {
			number: McBlockNumber(parent_stable_block.number.0 - 1),
			hash: mc_block_hash.clone(),
			slot: McSlotNumber(parent_stable_block.slot.0 - 1),
			timestamp: parent_stable_block.timestamp - 1,
			epoch: McEpochNumber(parent_stable_block.epoch.0),
		});

		let err = McHashInherentDataProvider::new_verification(
			mock_header(),
			Some(Slot::from(1)),
			30.into(),
			mc_block_hash.clone(),
			slot_duration,
			&block_data_source,
		)
		.await;
		assert!(err.is_err());
		assert_eq!(
			err.unwrap_err().to_string(),
			McStateReferenceRegressed(mc_block_hash, 30.into(), McBlockNumber(0), McBlockNumber(1))
				.to_string()
		);
	}

	pub fn mock_header() -> Header {
		Header::new(
			Default::default(),
			Default::default(),
			Default::default(),
			Default::default(),
			Digest { logs: McHashInherentDigest::from_mc_block_hash(McBlockHash([1; 32])) },
		)
	}
}
