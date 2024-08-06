use crate::inherent_data::{ProposalCIDP, VerifierCIDP};
use crate::tests::mock::{test_client, test_create_inherent_data_config};
use crate::tests::runtime_api_mock::mock_header;
use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionInputs;
use main_chain_follower_api::{block::MainchainBlock, mock_services::*};
use sidechain_domain::{McBlockHash, McBlockNumber, McEpochNumber, McSlotNumber};
use sp_consensus_aura::Slot;
use sp_core::H256;
use sp_inherents::CreateInherentDataProviders;
use sp_inherents::{InherentData, InherentDataProvider};
use sp_timestamp::Timestamp;
use std::env;

#[tokio::test]
async fn block_proposal_cidp_should_be_created_correctly() {
	env::set_var(
		"SIDECHAIN_BLOCK_BENEFICIARY",
		"0x0000000000000000000000000000000000000000000000000000000000000001",
	);

	let inherent_data_providers = ProposalCIDP::new(
		test_create_inherent_data_config(),
		test_client(),
		TestDataSources::new().into(),
	)
	.create_inherent_data_providers(H256::zero(), ())
	.await
	.unwrap();

	#[cfg(not(feature = "block-beneficiary"))]
	let (slot, timestamp, mc_hash, ariadne_data) = inherent_data_providers;
	#[cfg(feature = "block-beneficiary")]
	let (slot, timestamp, mc_hash, ariadne_data, block_beneficiary) = inherent_data_providers;
	let mut inherent_data = InherentData::new();
	slot.provide_inherent_data(&mut inherent_data).await.unwrap();
	timestamp.provide_inherent_data(&mut inherent_data).await.unwrap();
	mc_hash.provide_inherent_data(&mut inherent_data).await.unwrap();
	ariadne_data.provide_inherent_data(&mut inherent_data).await.unwrap();
	#[cfg(feature = "block-beneficiary")]
	block_beneficiary.provide_inherent_data(&mut inherent_data).await.unwrap();
	assert_eq!(
		inherent_data
			.get_data::<Slot>(&sp_consensus_aura::inherents::INHERENT_IDENTIFIER)
			.unwrap(),
		Some(Slot::from(30))
	);
	assert_eq!(
		inherent_data.get_data::<Timestamp>(&sp_timestamp::INHERENT_IDENTIFIER).unwrap(),
		Some(Timestamp::new(
			test_create_inherent_data_config().time_source.get_current_time_millis()
		))
	);
	assert_eq!(
		inherent_data
			.get_data::<McBlockHash>(&sidechain_mc_hash::INHERENT_IDENTIFIER)
			.unwrap(),
		MockBlockDataSource::default()
			.get_all_stable_blocks()
			.first()
			.map(|b| b.hash.clone())
	);
	assert!(inherent_data
		.get_data::<AuthoritySelectionInputs>(&sp_session_validator_management::INHERENT_IDENTIFIER)
		.unwrap()
		.is_some());
}

#[tokio::test]
async fn block_verification_cidp_should_be_created_correctly() {
	let mut block_data_source = MockBlockDataSource::default();
	let parent_stable_block = block_data_source.get_all_stable_blocks().first().unwrap().clone();
	let mc_block_hash = McBlockHash([2; 32]);
	block_data_source.push_stable_block(MainchainBlock {
		number: McBlockNumber(parent_stable_block.number.0 + 5),
		hash: mc_block_hash.clone(),
		slot: McSlotNumber(parent_stable_block.slot.0 + 100),
		timestamp: parent_stable_block.timestamp + 101,
		epoch: McEpochNumber(parent_stable_block.epoch.0),
	});
	let data_sources = TestDataSources::new().with_block_data_source(block_data_source).into();

	let create_inherent_data_config = test_create_inherent_data_config();

	let verifier_cidp =
		VerifierCIDP::new(create_inherent_data_config.clone(), test_client(), data_sources);

	let inherent_data_providers = verifier_cidp
		.create_inherent_data_providers(mock_header().hash(), (30.into(), mc_block_hash.clone()))
		.await
		.unwrap();
	let (timestamp, mc_hash, ariadne_data_provider) = inherent_data_providers;
	let mut inherent_data = InherentData::new();
	timestamp.provide_inherent_data(&mut inherent_data).await.unwrap();
	ariadne_data_provider.provide_inherent_data(&mut inherent_data).await.unwrap();

	assert_eq!(
		inherent_data.get_data::<Timestamp>(&sp_timestamp::INHERENT_IDENTIFIER).unwrap(),
		Some(Timestamp::new(create_inherent_data_config.time_source.get_current_time_millis()))
	);
	assert_eq!(mc_hash.mc_block.hash, mc_block_hash);
	assert!(inherent_data
		.get_data::<AuthoritySelectionInputs>(&sp_session_validator_management::INHERENT_IDENTIFIER)
		.unwrap()
		.is_some());
}
