mod rpc_mock;
mod runtime_api_mock;

use super::SidechainRpc;
use super::*;
use crate::mock::Block;
use crate::SidechainRpcApiServer;
use rpc_mock::*;

mod get_status_tests {
	use super::*;
	use mock::SidechainRpcDataSourceMock;
	use pretty_assertions::assert_eq;
	use sidechain_domain::mainchain_epoch::{Duration, MainchainEpochConfig};
	use sidechain_domain::*;
	use sp_consensus_slots::SlotDuration;

	#[tokio::test]
	async fn should_return_correct_status() {
		let mc_epoch_config = MainchainEpochConfig {
			first_epoch_timestamp_millis: Timestamp::from_unix_millis(1_000_000_000),
			epoch_duration_millis: Duration::from_millis(120_000),
			first_epoch_number: 50,
			first_slot_number: 501,
		};
		let mainchain_block = MainchainBlock {
			epoch: McEpochNumber(99),
			slot: McSlotNumber(2000),
			..Default::default()
		};
		let sidechain_rpc_data_source =
			Arc::new(SidechainRpcDataSourceMock::<ErrorObjectOwned>::new(mainchain_block.clone()));
		let slot_duration = SlotDuration::from_millis(60);
		let slots_per_epoch = 10;

		let client = Arc::new(TestApi {
			runtime_api: TestRuntimeApi {
				sidechain_status: vec![],
				slot_duration,
				slots_per_epoch,
			},
		});
		let current_time_millis: u64 = 1_000_000_000_000;
		let time_source = Arc::new(MockedTimeSource { current_time_millis });

		let api = SidechainRpc::new(
			client,
			mc_epoch_config.clone(),
			sidechain_rpc_data_source,
			time_source,
		);
		let current_epoch = current_time_millis / (slot_duration.as_millis() * slots_per_epoch);
		let status_response = api.get_status().await;

		assert_eq!(
			status_response.expect("Response should not be an error"),
			GetStatusResponse {
				sidechain: SidechainData {
					epoch: current_epoch,
					slot: current_time_millis / slot_duration.as_millis(),
					next_epoch_timestamp: Timestamp::from_unix_millis(
						(current_epoch + 1) * slots_per_epoch * slot_duration.as_millis()
					)
				},
				mainchain: MainchainData {
					epoch: mainchain_block.epoch.0,
					slot: mainchain_block.slot.0,
					next_epoch_timestamp: Timestamp::from_unix_millis(
						mc_epoch_config.first_epoch_timestamp_millis.unix_millis()
							+ mc_epoch_config.epoch_duration_millis.millis() * 100
					)
				}
			}
		)
	}

	#[tokio::test]
	async fn get_params_return_chain_parameters() {
		let client = Arc::new(TestApi::default());
		let irrelevant_epoch_config = MainchainEpochConfig {
			first_epoch_timestamp_millis: Timestamp::from_unix_millis(1_000_000_000),
			epoch_duration_millis: Duration::from_millis(120_000),
			first_epoch_number: 50,
			first_slot_number: 501,
		};
		let mainchain_block = MainchainBlock {
			epoch: McEpochNumber(99),
			slot: McSlotNumber(2000),
			..Default::default()
		};

		let api = SidechainRpc::new(
			client,
			irrelevant_epoch_config,
			Arc::new(SidechainRpcDataSourceMock::<ErrorObjectOwned>::new(mainchain_block)),
			Arc::new(MockedTimeSource { current_time_millis: 0 }),
		);
		let response = api.get_params();

		assert_eq!(
			response.expect("Response should not be an error"),
			crate::tests::mock::mock_sidechain_params(),
		)
	}
}
