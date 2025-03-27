#![allow(non_upper_case_globals)]

use crate::inherent_data::*;
use crate::*;
use mainchain_epoch::*;
use pretty_assertions::assert_eq;
use sidechain_domain::*;
use sp_api::ProvideRuntimeApi;
use sp_inherents::{InherentData, InherentDataProvider};
use std::collections::HashMap;

pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

pub type Hash = <Block as sp_runtime::traits::Block>::Hash;

#[derive(Default)]
struct MockBlockParticipationDataSource {
	stake_distributions: HashMap<McEpochNumber, StakeDistribution>,
}

#[async_trait::async_trait]
impl BlockParticipationDataSource for MockBlockParticipationDataSource {
	async fn get_stake_pool_delegation_distribution_for_pools(
		&self,
		epoch: McEpochNumber,
		_pool_hashes: &[MainchainKeyHash],
	) -> Result<StakeDistribution, Box<dyn std::error::Error + Send + Sync>> {
		Ok(self
			.stake_distributions
			.get(&epoch)
			.cloned()
			.expect("Mock called with unexpected epoch"))
	}
}

#[derive(Debug, Clone)]
struct TestApi {
	payout_slot: Option<Slot>,
	blocks_produced_up_to_slot: Vec<(Slot, BlockProducer)>,
}

impl ProvideRuntimeApi<Block> for TestApi {
	type Api = Self;

	fn runtime_api(&self) -> sp_api::ApiRef<Self::Api> {
		(*self).clone().into()
	}
}

type BlockProducer = Option<MainchainKeyHash>;
type Beneficiary = DelegatorKey;

const TEST_INHERENT_ID: InherentIdentifier = [42; 8];

sp_api::mock_impl_runtime_apis! {
	impl BlockParticipationApi<Block, BlockProducer> for TestApi {

		fn should_release_data(_slot: Slot) -> Option<Slot> {
			self.payout_slot
		}
		fn blocks_produced_up_to_slot(_slot: Slot) -> Vec<(Slot, BlockProducer)> {
			self.blocks_produced_up_to_slot.clone()
		}
		fn target_inherent_id() -> InherentIdentifier {
			TEST_INHERENT_ID
		}
	}
}

const producer1: MainchainKeyHash = MainchainKeyHash([1; 28]);
const producer2: MainchainKeyHash = MainchainKeyHash([2; 28]);
const producer3: MainchainKeyHash = MainchainKeyHash([3; 28]);
const producer4: MainchainKeyHash = MainchainKeyHash([4; 28]);
const producer5: MainchainKeyHash = MainchainKeyHash([5; 28]);

const delegator1: DelegatorKey = DelegatorKey::StakeKeyHash([11; 28]);
const delegator2: DelegatorKey = DelegatorKey::StakeKeyHash([12; 28]);
const delegator3: DelegatorKey = DelegatorKey::StakeKeyHash([13; 28]);
const delegator4: DelegatorKey = DelegatorKey::StakeKeyHash([14; 28]);
const delegator5: DelegatorKey = DelegatorKey::StakeKeyHash([15; 28]);

#[tokio::test]
async fn provides_data_when_api_returns_a_slot() {
	let payout_slot = Slot::from(1000);
	let parent_hash = Hash::from([2; 32]);
	let current_slot = Slot::from(10);
	let mc_epoch_config = MainchainEpochConfig {
		first_epoch_timestamp_millis: Timestamp::from_unix_millis(0),
		first_epoch_number: 0,
		epoch_duration_millis: Duration::from_millis(10000),
		first_slot_number: 0,
		slot_duration_millis: Duration::from_millis(100),
	};
	let slot_duration = SlotDuration::from_millis(1000);
	let client = TestApi {
		payout_slot: Some(payout_slot),
		blocks_produced_up_to_slot: vec![
			// epoch 47
			(Slot::from(490), Some(producer1)),
			(Slot::from(491), Some(producer2)),
			(Slot::from(492), Some(producer1)),
			(Slot::from(493), None),
			(Slot::from(494), Some(producer3)),
			(Slot::from(495), Some(producer4)),
			(Slot::from(496), None),
			(Slot::from(497), Some(producer5)),
			// epoch 97
			(Slot::from(990), Some(producer1)),
			(Slot::from(991), Some(producer2)),
			(Slot::from(992), Some(producer1)),
			(Slot::from(993), Some(producer2)),
			(Slot::from(994), Some(producer3)),
			(Slot::from(995), Some(producer4)),
		],
	};
	#[rustfmt::skip]
	let data_source = MockBlockParticipationDataSource {
		stake_distributions: [(
			McEpochNumber(47),
			StakeDistribution(
				[
                    (producer1, PoolDelegation { total_stake: StakeDelegation(10000), delegators: [
                        (delegator1, 9000u64.into()),
                        (delegator2, 1000u64.into()),
                    ].into() }),
                    (producer2, PoolDelegation { total_stake: StakeDelegation(100), delegators: [
                        (delegator3, 100u64.into()),
                    ].into() }),
                    (producer3, PoolDelegation { total_stake: StakeDelegation(9900), delegators: [
                        (delegator3, 9000u64.into()),
                        (delegator4, 900u64.into()),
                    ].into() }),
                    (producer4, PoolDelegation { total_stake: StakeDelegation(12000), delegators: [
                        (delegator5, 12000u64.into()),
                    ].into() }),
                    (producer5, PoolDelegation { total_stake: StakeDelegation(200), delegators: [
                        (delegator1, 200u64.into()),
                    ].into() }),
                ]
				.into(),
			),
		),
        (
			McEpochNumber(97),
			StakeDistribution(
				[
                    (producer1, PoolDelegation { total_stake: StakeDelegation(9000), delegators: [
                        (delegator1, 9000u64.into()),
                    ].into() }),
                    (producer2, PoolDelegation { total_stake: StakeDelegation(1100), delegators: [
                        (delegator2, 1000u64.into()),
                        (delegator3, 100u64.into()),
                    ].into() }),
                    (producer3, PoolDelegation { total_stake: StakeDelegation(9000), delegators: [
                        (delegator3, 9000u64.into()),
                    ].into() }),
                    (producer4, PoolDelegation { total_stake: StakeDelegation(12900), delegators: [
                        (delegator4, 900u64.into()),
                        (delegator5, 12000u64.into()),
                    ].into() }),
                    (producer5, PoolDelegation { total_stake: StakeDelegation(200), delegators: [
                        (delegator1, 200u64.into()),
                    ].into() }),
                ]
				.into(),
			),
		)]
		.into(),
	};

	let provider = BlockParticipationInherentDataProvider::<BlockProducer, Beneficiary>::new(
		&client,
		&data_source,
		parent_hash,
		current_slot,
		&mc_epoch_config,
		slot_duration,
	)
	.await
	.expect("Should succeed");

	let BlockParticipationInherentDataProvider::Active {
		target_inherent_id,
		block_production_data,
	} = provider
	else {
		panic!("Should be active")
	};

	assert_eq!(target_inherent_id, TEST_INHERENT_ID);
	assert_eq!(block_production_data.up_to_slot, payout_slot);

	#[rustfmt::skip]
	assert_eq!(
		block_production_data.producer_participation,
		vec![
			BlockProducerParticipationData {
				block_producer: None,
				block_count: 2,
				delegator_total_shares: 0,
				delegators: vec![]
			},
			BlockProducerParticipationData {
				block_producer: Some(producer1),
				block_count: 2,
				delegator_total_shares: 9000,
				delegators: vec![
                    DelegatorBlockParticipationData { id: delegator1, share: 9000 },
                ]
			},
			BlockProducerParticipationData {
				block_producer: Some(producer1),
				block_count: 2,
				delegator_total_shares: 10000,
				delegators: vec![
					DelegatorBlockParticipationData { id: delegator1, share: 9000 },
					DelegatorBlockParticipationData { id: delegator2, share: 1000 },
				]
			},
			BlockProducerParticipationData {
				block_producer: Some(producer2),
				block_count: 1,
				delegator_total_shares: 100,
				delegators: vec![
                    DelegatorBlockParticipationData { id: delegator3, share: 100 },
                ]
			},
			BlockProducerParticipationData {
				block_producer: Some(producer2),
				block_count: 2,
				delegator_total_shares: 1100,
				delegators: vec![
					DelegatorBlockParticipationData { id: delegator2, share: 1000 },
					DelegatorBlockParticipationData { id: delegator3, share: 100 },
				]
			},
			BlockProducerParticipationData {
				block_producer: Some(producer3),
				block_count: 1,
				delegator_total_shares: 9000,
				delegators: vec![
                    DelegatorBlockParticipationData { id: delegator3, share: 9000 },
                ]
			},
			BlockProducerParticipationData {
				block_producer: Some(producer3),
				block_count: 1,
				delegator_total_shares: 9900,
				delegators: vec![
					DelegatorBlockParticipationData { id: delegator3, share: 9000 },
					DelegatorBlockParticipationData { id: delegator4, share: 900 }
				]
			},
			BlockProducerParticipationData {
				block_producer: Some(producer4),
				block_count: 1,
				delegator_total_shares: 12000,
				delegators: vec![
                    DelegatorBlockParticipationData { id: delegator5, share: 12000 },
                ]
			},
			BlockProducerParticipationData {
				block_producer: Some(producer4),
				block_count: 1,
				delegator_total_shares: 12900,
				delegators: vec![
					DelegatorBlockParticipationData { id: delegator4, share: 900 },
					DelegatorBlockParticipationData { id: delegator5, share: 12000 }
				]
			},
			BlockProducerParticipationData {
				block_producer: Some(producer5),
				block_count: 1,
				delegator_total_shares: 200,
				delegators: vec![
                    DelegatorBlockParticipationData { id: delegator1, share: 200 }
                ]
			}
		]
	);
}

#[tokio::test]
async fn skips_providing_data_if_api_returns_none() {
	let client = TestApi { payout_slot: None, blocks_produced_up_to_slot: vec![] };
	let data_source = MockBlockParticipationDataSource::default();
	let parent_hash = Hash::from([2; 32]);
	let current_slot = Slot::from(10);
	let mc_epoch_config = MainchainEpochConfig {
		first_epoch_timestamp_millis: Timestamp::from_unix_millis(0),
		first_epoch_number: 0,
		epoch_duration_millis: Duration::from_millis(10000),
		first_slot_number: 0,
		slot_duration_millis: Duration::from_millis(1000),
	};
	let slot_duration = SlotDuration::from_millis(1000);

	let provider = BlockParticipationInherentDataProvider::<BlockProducer, Beneficiary>::new(
		&client,
		&data_source,
		parent_hash,
		current_slot,
		&mc_epoch_config,
		slot_duration,
	)
	.await
	.expect("Should succeed");

	assert_eq!(provider, BlockParticipationInherentDataProvider::Inert);
}

#[tokio::test]
async fn returns_error_if_producer_missing_in_stake_distribution() {
	let client = TestApi {
		payout_slot: Some(Slot::from(900)),
		blocks_produced_up_to_slot: vec![
			(Slot::from(490), Some(producer1)),
			(Slot::from(491), Some(producer1)),
		],
	};
	let data_source = MockBlockParticipationDataSource {
		stake_distributions: [(McEpochNumber(47), StakeDistribution([].into()))].into(),
	};
	let parent_hash = Hash::from([2; 32]);
	let current_slot = Slot::from(10);
	let mc_epoch_config = MainchainEpochConfig {
		first_epoch_timestamp_millis: Timestamp::from_unix_millis(0),
		first_epoch_number: 0,
		epoch_duration_millis: Duration::from_millis(10000),
		first_slot_number: 0,
		slot_duration_millis: Duration::from_millis(1000),
	};
	let slot_duration = SlotDuration::from_millis(1000);

	let err = BlockParticipationInherentDataProvider::<BlockProducer, Beneficiary>::new(
		&client,
		&data_source,
		parent_hash,
		current_slot,
		&mc_epoch_config,
		slot_duration,
	)
	.await
	.expect_err("Should return error");

	match err {
		InherentDataCreationError::DataMissing(epoch, producer) => {
			assert_eq!(epoch, McEpochNumber(47));
			assert_eq!(producer, Some(producer1));
		},
		err => panic!("Unexpected error: {err:?}"),
	}
}

#[tokio::test]
async fn idp_provides_two_inherent_data_sets() {
	let production_data: BlockProductionData<BlockProducer, Beneficiary> =
		BlockProductionData::new(Slot::from(11), vec![]);
	let provider = BlockParticipationInherentDataProvider::Active {
		target_inherent_id: TEST_INHERENT_ID,
		block_production_data: production_data.clone(),
	};

	let mut inherent_data = InherentData::new();
	provider.provide_inherent_data(&mut inherent_data).await.unwrap();

	assert_eq!(inherent_data.get_data::<Slot>(&INHERENT_IDENTIFIER).unwrap(), Some(Slot::from(11)));
	assert_eq!(
		inherent_data
			.get_data::<BlockProductionData<BlockProducer, Beneficiary>>(&TEST_INHERENT_ID)
			.unwrap(),
		Some(production_data)
	);
}
