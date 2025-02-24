use crate::{BlockAuthorInherentProvider, BlockProductionLogApi};
use sp_api::ApiRef;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block as BlockT;

pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

type Member = u32;
type Author = u64;

#[derive(Clone, Default)]
struct TestApi {
	author: Member,
}

impl ProvideRuntimeApi<Block> for TestApi {
	type Api = Self;
	fn runtime_api(&self) -> ApiRef<Self::Api> {
		(*self).clone().into()
	}
}

sp_api::mock_impl_runtime_apis! {
	impl BlockProductionLogApi<Block, Member> for TestApi {

		fn get_current_author() -> Member {
			self.author
		}
	}
}

#[test]
fn provides_author_based_on_runtime_api() {
	let mock_api = TestApi { author: 102 };

	let provider =
		BlockAuthorInherentProvider::<Author>::new(&mock_api, <Block as BlockT>::Hash::default())
			.expect("Should not fail");

	assert_eq!(provider.author, mock_api.author.into());
}
