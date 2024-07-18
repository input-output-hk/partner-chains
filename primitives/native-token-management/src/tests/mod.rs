pub(crate) mod runtime_api_mock;

#[cfg(feature = "std")]
mod inherent_provider {
	use super::runtime_api_mock::*;
	use crate::inherent_provider::*;
	use crate::INHERENT_IDENTIFIER;
	use main_chain_follower_api::mock_services::MockNativeTokenDataSource;
	use sidechain_domain::*;
	use sidechain_mc_hash::MC_HASH_DIGEST_ID;
	use sp_inherents::InherentData;
	use sp_inherents::InherentDataProvider;
	use sp_runtime::testing::Digest;
	use sp_runtime::testing::DigestItem;
	use std::sync::Arc;

	#[tokio::test]
	async fn correctly_fetches_total_transfer_between_two_hashes() {
		let parent_number = 1; // not genesis

		let mc_hash = McBlockHash([1; 32]);
		let parent_hash = Hash::from([2; 32]);
		let parent_mc_hash = Some(McBlockHash([3; 32]));
		let total_transfered = 103;

		let data_source =
			create_data_source(parent_mc_hash.clone(), mc_hash.clone(), total_transfered);
		let client = create_client(parent_hash, parent_mc_hash, parent_number);

		let inherent_provider = NativeTokenManagementInherentDataProvider::new(
			client,
			&data_source,
			mc_hash,
			parent_hash,
		)
		.await
		.expect("Should not fail");

		assert_eq!(inherent_provider.token_amount.0, total_transfered)
	}

	#[tokio::test]
	async fn fetches_with_no_lower_bound_when_parent_is_genesis() {
		let parent_number = 0; // genesis

		let mc_hash = McBlockHash([1; 32]);
		let parent_hash = Hash::from([2; 32]);
		let parent_mc_hash = None; // genesis doesn't refer to any mc hash
		let total_transfered = 103;

		let data_source =
			create_data_source(parent_mc_hash.clone(), mc_hash.clone(), total_transfered);
		let client = create_client(parent_hash, parent_mc_hash, parent_number);

		let inherent_provider = NativeTokenManagementInherentDataProvider::new(
			client,
			&data_source,
			mc_hash,
			parent_hash,
		)
		.await
		.expect("Should not fail");

		assert_eq!(inherent_provider.token_amount.0, total_transfered)
	}

	#[tokio::test]
	async fn defaults_to_zero_when_no_data() {
		let parent_number = 1;

		let mc_hash = McBlockHash([1; 32]);
		let parent_hash = Hash::from([2; 32]);
		let parent_mc_hash = Some(McBlockHash([3; 32]));

		let data_source = MockNativeTokenDataSource::new([].into());
		let client = create_client(parent_hash, parent_mc_hash, parent_number);

		let inherent_provider = NativeTokenManagementInherentDataProvider::new(
			client,
			&data_source,
			mc_hash,
			parent_hash,
		)
		.await
		.expect("Should not fail");

		assert_eq!(inherent_provider.token_amount.0, 0)
	}

	#[tokio::test]
	async fn correctly_puts_data_into_inherent_data_structure() {
		let token_amount = 1234;

		let mut inherent_data = InherentData::new();

		let inherent_provider = NativeTokenManagementInherentDataProvider {
			token_amount: NativeTokenAmount(token_amount),
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
		total_transfered: u64,
	) -> MockNativeTokenDataSource {
		let total_transfered = NativeTokenAmount(total_transfered);
		MockNativeTokenDataSource::new([((parent_mc_hash, mc_hash), total_transfered)].into())
	}

	fn create_client(
		parent_hash: Hash,
		parent_mc_hash: Option<McBlockHash>,
		parent_number: u32,
	) -> Arc<TestApi> {
		Arc::new(TestApi {
			headers: [(
				parent_hash.clone(),
				Header {
					digest: Digest {
						logs: match parent_mc_hash {
							None => vec![],
							Some(parent_mc_hash) => vec![DigestItem::PreRuntime(
								MC_HASH_DIGEST_ID,
								parent_mc_hash.0.to_vec(),
							)],
						},
					},
					extrinsics_root: Default::default(),
					number: parent_number,
					parent_hash: parent_hash.clone(),
					state_root: Default::default(),
				},
			)]
			.into(),
		})
	}
}
