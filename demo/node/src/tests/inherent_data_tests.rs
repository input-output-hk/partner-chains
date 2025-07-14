use crate::inherent_data::{ProposalCIDP, VerifierCIDP};
use crate::tests::mock::{test_client, test_create_inherent_data_config};
use crate::tests::runtime_api_mock;
use crate::tests::runtime_api_mock::{TestApi, mock_header};
use authority_selection_inherents::{
	AuthoritySelectionInputs, mock::MockAuthoritySelectionDataSource,
};
use hex_literal::hex;
use partner_chains_demo_runtime::BlockAuthor;
use partner_chains_mock_data_sources::GovernedMapDataSourceMock;
use partner_chains_mock_data_sources::StakeDistributionDataSourceMock;
use sidechain_domain::{
	DelegatorKey, MainchainBlock, McBlockHash, McBlockNumber, McEpochNumber, McSlotNumber,
	NativeTokenAmount, ScEpochNumber,
};
use sidechain_mc_hash::mock::MockMcHashDataSource;
use sp_block_participation::BlockProductionData;
use sp_consensus_aura::Slot;
use sp_core::ecdsa;
use sp_inherents::CreateInherentDataProviders;
use sp_inherents::{InherentData, InherentDataProvider};
use sp_native_token_management::mock::MockNativeTokenDataSource;
use sp_timestamp::Timestamp;
use std::sync::Arc;

#[tokio::test]
async fn block_proposal_cidp_should_be_created_correctly() {
	let native_token_data_source = MockNativeTokenDataSource::new(
		[((Some(McBlockHash([0; 32])), McBlockHash([1; 32])), NativeTokenAmount(1000))].into(),
	);
	let parent_stable_block = MainchainBlock {
		number: McBlockNumber(0),
		hash: McBlockHash([0; 32]),
		epoch: McEpochNumber(2),
		slot: McSlotNumber(2),
		timestamp: 3,
	};
	let stable_block = MainchainBlock {
		number: McBlockNumber(1),
		hash: McBlockHash([1; 32]),
		epoch: McEpochNumber(2),
		slot: McSlotNumber(3),
		timestamp: 4,
	};
	let mc_hash_data_source =
		MockMcHashDataSource::from(vec![parent_stable_block.clone(), stable_block.clone()]);

	let inherent_data_providers = ProposalCIDP::new(
		test_create_inherent_data_config(),
		TestApi::new(ScEpochNumber(2))
			.with_headers([(mock_header().hash(), mock_header())])
			.into(),
		Arc::new(mc_hash_data_source),
		Arc::new(MockAuthoritySelectionDataSource::default()),
		Arc::new(native_token_data_source),
		Arc::new(StakeDistributionDataSourceMock::new()),
		Arc::new(GovernedMapDataSourceMock::default()),
	)
	.create_inherent_data_providers(mock_header().hash(), ())
	.await
	.unwrap();

	let mut inherent_data = InherentData::new();
	inherent_data_providers.provide_inherent_data(&mut inherent_data).await.unwrap();

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
		Some(McBlockHash([1; 32]))
	);
	assert!(
		inherent_data
			.get_data::<AuthoritySelectionInputs>(
				&sp_session_validator_management::INHERENT_IDENTIFIER
			)
			.unwrap()
			.is_some()
	);
	assert!(
		inherent_data
			.get_data::<NativeTokenAmount>(&sp_native_token_management::INHERENT_IDENTIFIER)
			.unwrap()
			.is_some()
	);
	assert_eq!(
		inherent_data
			.get_data::<BlockAuthor>(&sp_block_production_log::INHERENT_IDENTIFIER)
			.unwrap(),
		Some(BlockAuthor::ProBono(
			ecdsa::Public::from_raw(hex!(
				"000000000000000000000000000000000000000000000000000000000000000001"
			))
			.into()
		))
	);
	assert_eq!(
		inherent_data
			.get_data::<Slot>(&sp_block_participation::INHERENT_IDENTIFIER)
			.unwrap(),
		Some(Slot::from(30))
	);
	assert_eq!(
		inherent_data
			.get_data::<BlockProductionData<BlockAuthor, DelegatorKey>>(
				&runtime_api_mock::TEST_TARGET_INHERENT_ID
			)
			.unwrap(),
		Some(BlockProductionData::new(Slot::from(30), vec![]))
	);
}

#[tokio::test]
async fn block_verification_cidp_should_be_created_correctly() {
	let parent_stable_block = MainchainBlock {
		number: McBlockNumber(1),
		hash: McBlockHash([1; 32]),
		epoch: McEpochNumber(2),
		slot: McSlotNumber(3),
		timestamp: 4,
	};
	let mc_block_hash = McBlockHash([2; 32]);
	let native_token_data_source = MockNativeTokenDataSource::new(
		[((None, mc_block_hash.clone()), NativeTokenAmount(1000))].into(),
	);
	let mc_hash_data_source = MockMcHashDataSource::from(vec![MainchainBlock {
		number: McBlockNumber(parent_stable_block.number.0 + 5),
		hash: mc_block_hash.clone(),
		slot: McSlotNumber(parent_stable_block.slot.0 + 100),
		timestamp: parent_stable_block.timestamp + 101,
		epoch: McEpochNumber(parent_stable_block.epoch.0),
	}]);

	let create_inherent_data_config = test_create_inherent_data_config();

	let verifier_cidp = VerifierCIDP::new(
		create_inherent_data_config.clone(),
		test_client(),
		Arc::new(mc_hash_data_source),
		Arc::new(MockAuthoritySelectionDataSource::default()),
		Arc::new(native_token_data_source),
		Arc::new(StakeDistributionDataSourceMock::new()),
		Arc::new(GovernedMapDataSourceMock::default()),
	);

	let inherent_data_providers = verifier_cidp
		.create_inherent_data_providers(mock_header().hash(), (30.into(), mc_block_hash))
		.await
		.unwrap();
	let mut inherent_data = InherentData::new();
	inherent_data_providers.provide_inherent_data(&mut inherent_data).await.unwrap();

	assert_eq!(
		inherent_data.get_data::<Timestamp>(&sp_timestamp::INHERENT_IDENTIFIER).unwrap(),
		Some(Timestamp::new(create_inherent_data_config.time_source.get_current_time_millis()))
	);
	assert!(
		inherent_data
			.get_data::<AuthoritySelectionInputs>(
				&sp_session_validator_management::INHERENT_IDENTIFIER
			)
			.unwrap()
			.is_some()
	);
	assert!(
		inherent_data
			.get_data::<NativeTokenAmount>(&sp_native_token_management::INHERENT_IDENTIFIER)
			.unwrap()
			.is_some()
	);
	assert_eq!(
		inherent_data
			.get_data::<BlockAuthor>(&sp_block_production_log::INHERENT_IDENTIFIER)
			.unwrap(),
		Some(BlockAuthor::ProBono(
			ecdsa::Public::from_raw(hex!(
				"000000000000000000000000000000000000000000000000000000000000000001"
			))
			.into()
		))
	);
	assert_eq!(
		inherent_data
			.get_data::<Slot>(&sp_block_participation::INHERENT_IDENTIFIER)
			.unwrap(),
		Some(Slot::from(30))
	);
	assert_eq!(
		inherent_data
			.get_data::<BlockProductionData<BlockAuthor, DelegatorKey>>(
				&runtime_api_mock::TEST_TARGET_INHERENT_ID
			)
			.unwrap(),
		Some(BlockProductionData::new(Slot::from(30), vec![]))
	);
}
