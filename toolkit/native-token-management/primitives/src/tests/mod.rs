pub(crate) mod runtime_api_mock;

#[cfg(feature = "std")]
mod inherent_provider {
	use super::runtime_api_mock::*;
	use crate::inherent_provider::mock::*;
	use crate::inherent_provider::*;
	use crate::{InherentError, MainChainScripts, INHERENT_IDENTIFIER};
	use sidechain_domain::*;
	use sp_inherents::{InherentData, InherentDataProvider};
	use std::sync::Arc;

	#[test]
	fn error_message_formatting() {
		assert_eq!(
			InherentError::TokenTransferNotHandled(NativeTokenAmount(3u128)).to_string(),
			"Inherent missing for token transfer of 3 tokens"
		);
		assert_eq!(
			InherentError::IncorrectTokenNumberTransfered(
				NativeTokenAmount(13u128),
				NativeTokenAmount(7u128)
			)
			.to_string(),
			"Incorrect token transfer amount: expected 13, got 7 tokens"
		);
		assert_eq!(
			InherentError::UnexpectedTokenTransferInherent(NativeTokenAmount(13u128)).to_string(),
			"Unexpected transfer of 13 tokens"
		);
	}

	#[tokio::test]
	async fn correctly_fetches_total_transfer_between_two_hashes() {
		let mc_hash = McBlockHash([1; 32]);
		let parent_hash = Hash::from([2; 32]);
		let parent_mc_hash = Some(McBlockHash([3; 32]));
		let total_transfered = 103;

		let data_source =
			create_data_source(parent_mc_hash.clone(), mc_hash.clone(), total_transfered);
		let main_chain_scripts = Some(MainChainScripts::default());
		let client = create_client(main_chain_scripts);

		let inherent_provider = NativeTokenManagementInherentDataProvider::new(
			client,
			&data_source,
			mc_hash,
			parent_mc_hash,
			parent_hash,
		)
		.await
		.expect("Should not fail");

		assert_eq!(inherent_provider.token_amount, Some(total_transfered.into()))
	}

	#[tokio::test]
	async fn fetches_with_no_lower_bound_when_parent_is_genesis() {
		let mc_hash = McBlockHash([1; 32]);
		let parent_hash = Hash::from([2; 32]);
		let parent_mc_hash = None; // genesis doesn't refer to any mc hash
		let total_transfered = 103;

		let data_source =
			create_data_source(parent_mc_hash.clone(), mc_hash.clone(), total_transfered);
		let main_chain_scripts = Some(MainChainScripts::default());
		let client = create_client(main_chain_scripts);

		let inherent_provider = NativeTokenManagementInherentDataProvider::new(
			client,
			&data_source,
			mc_hash,
			parent_mc_hash,
			parent_hash,
		)
		.await
		.expect("Should not fail");

		assert_eq!(inherent_provider.token_amount, Some(total_transfered.into()))
	}

	#[tokio::test]
	async fn defaults_to_none_when_no_data() {
		let mc_hash = McBlockHash([1; 32]);
		let parent_hash = Hash::from([2; 32]);
		let parent_mc_hash = Some(McBlockHash([3; 32]));

		let data_source = MockNativeTokenDataSource::new([].into());
		let main_chain_scripts = Some(MainChainScripts::default());
		let client = create_client(main_chain_scripts);

		let inherent_provider = NativeTokenManagementInherentDataProvider::new(
			client,
			&data_source,
			mc_hash,
			parent_mc_hash,
			parent_hash,
		)
		.await
		.expect("Should not fail");

		assert_eq!(inherent_provider.token_amount, None);
	}

	#[tokio::test]
	async fn defaults_to_none_when_scripts_are_unset() {
		let mc_hash = McBlockHash([1; 32]);
		let parent_hash = Hash::from([2; 32]);
		let parent_mc_hash = Some(McBlockHash([3; 32]));
		let total_transfered = 103;

		let data_source =
			create_data_source(parent_mc_hash.clone(), mc_hash.clone(), total_transfered);
		let main_chain_scripts = None;
		let client = create_client(main_chain_scripts);

		let inherent_provider = NativeTokenManagementInherentDataProvider::new(
			client,
			&data_source,
			mc_hash,
			parent_mc_hash,
			parent_hash,
		)
		.await
		.expect("Should not fail");

		assert_eq!(inherent_provider.token_amount, None)
	}

	#[tokio::test]
	async fn correctly_puts_data_into_inherent_data_structure() {
		let token_amount = 1234;

		let mut inherent_data = InherentData::new();

		let inherent_provider = NativeTokenManagementInherentDataProvider {
			token_amount: Some(NativeTokenAmount(token_amount)),
		};

		inherent_provider.provide_inherent_data(&mut inherent_data).await.unwrap();

		assert_eq!(
			inherent_data
				.get_data::<NativeTokenAmount>(&INHERENT_IDENTIFIER)
				.unwrap()
				.unwrap()
				.0,
			token_amount
		)
	}

	fn create_data_source(
		parent_mc_hash: Option<McBlockHash>,
		mc_hash: McBlockHash,
		total_transfered: u128,
	) -> MockNativeTokenDataSource {
		let total_transfered = NativeTokenAmount(total_transfered);
		MockNativeTokenDataSource::new([((parent_mc_hash, mc_hash), total_transfered)].into())
	}

	fn create_client(main_chain_scripts: Option<MainChainScripts>) -> Arc<TestApi> {
		Arc::new(TestApi { main_chain_scripts })
	}
}
