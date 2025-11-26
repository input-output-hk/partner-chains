mod rpc_mock;

use super::SidechainRpc;
use super::*;
use crate::SidechainRpcApiServer;
use crate::mock::Block;
use rpc_mock::*;

mod get_status_tests {
	use super::*;
	use crate::GetParamsOutput;
	use crate::mock::*;
	use mock::SidechainRpcDataSourceMock;
	use pretty_assertions::assert_eq;
	use sp_core::offchain::Timestamp;

	#[tokio::test]
	async fn should_return_correct_status() {
		let mc_epoch_config = mock_mc_epoch_config();
		let mainchain_block = mock_mainchain_block();
		let sidechain_rpc_data_source =
			Arc::new(SidechainRpcDataSourceMock::new(mainchain_block.clone()));
		let epoch_duration = 6000;
		let slot_duration = 10;

		let client = Arc::new(TestApi {
			epoch_duration,
			#[cfg(feature = "legacy-slotapi-compat")]
			slot_duration: Some(slot_duration),
		});
		let current_time_millis: u64 = 1_000_000_000_000;
		let time_source = Arc::new(MockedTimeSource { current_time_millis });

		let api = SidechainRpc::new(
			client,
			mc_epoch_config.clone(),
			sidechain_rpc_data_source,
			time_source,
		);
		let current_epoch = current_time_millis / epoch_duration;
		let status_response = api.get_status().await;

		assert_eq!(
			status_response.expect("Response should not be an error"),
			GetStatusResponse {
				sidechain: SidechainData {
					epoch: current_epoch,
					#[cfg(feature = "legacy-slotapi-compat")]
					slot: Some(current_time_millis / slot_duration),
					next_epoch_timestamp: Timestamp::from_unix_millis(
						(current_epoch + 1) * epoch_duration
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

	#[cfg(feature = "legacy-slotapi-compat")]
	#[tokio::test]
	async fn should_omit_slot_number_when_api_not_available_in_compat_mode() {
		let api = SidechainRpc::<_, Block>::new(
			Arc::new(TestApi::default()),
			mock_mc_epoch_config(),
			Arc::new(SidechainRpcDataSourceMock::new(mock_mainchain_block())),
			Arc::new(MockedTimeSource { current_time_millis: 1_000_000_000_000 }),
		);

		let status_response = api.get_status().await.expect("should succeed");

		assert_eq!(status_response.sidechain.slot, None);
	}

	#[tokio::test]
	async fn get_params_return_chain_parameters() {
		let client = Arc::new(TestApi::default());
		let irrelevant_epoch_config = mock_mc_epoch_config();
		let mainchain_block = mock_mainchain_block();

		let api = SidechainRpc::new(
			client,
			irrelevant_epoch_config,
			Arc::new(SidechainRpcDataSourceMock::new(mainchain_block)),
			Arc::new(MockedTimeSource { current_time_millis: 0 }),
		);
		let response = api.get_params();

		assert_eq!(
			response.expect("Response should not be an error"),
			GetParamsOutput { genesis_utxo: crate::tests::mock::mock_utxo_id() },
		)
	}
}
