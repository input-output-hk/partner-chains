use crate::BlockProductionInherentDataV1;
use crate::{BlockAuthorInherentProvider, BlockProductionLogApi};
use sp_api::ApiRef;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block as BlockT;

pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

type Author = u64;

#[derive(Clone, Default)]
struct TestApi {
	author: Option<Author>,
}

impl ProvideRuntimeApi<Block> for TestApi {
	type Api = Self;
	fn runtime_api(&self) -> ApiRef<'_, Self::Api> {
		(*self).clone().into()
	}
}

sp_api::mock_impl_runtime_apis! {
	impl BlockProductionLogApi<Block, Author, u64> for TestApi {
		fn get_author(_moment: &u64) -> Option<Author> {
			self.author
		}
	}
}

#[test]
fn provides_author_when_runtime_api_returns_one() {
	let mock_api = TestApi { author: Some(102) };

	let provider = BlockAuthorInherentProvider::<u64, Author>::new(
		&mock_api,
		<Block as BlockT>::Hash::default(),
		1000,
	)
	.expect("Should not fail");

	assert_eq!(
		provider.data,
		Some(BlockProductionInherentDataV1 { moment: 1000, block_producer_id: 102 })
	);
}

#[test]
fn skips_providing_author_when_runtime_api_returns_none() {
	let mock_api = TestApi { author: None };

	let provider = BlockAuthorInherentProvider::<u64, Author>::new(
		&mock_api,
		<Block as BlockT>::Hash::default(),
		1000,
	)
	.expect("Should not fail");

	assert_eq!(provider.data, None);
}
