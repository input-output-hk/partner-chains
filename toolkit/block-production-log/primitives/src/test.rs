use crate::{BlockAuthorInherentProvider, BlockProductionLogApi};
use sidechain_slots::Slot;
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
	author: Option<Member>,
}

impl ProvideRuntimeApi<Block> for TestApi {
	type Api = Self;
	fn runtime_api(&self) -> ApiRef<Self::Api> {
		(*self).clone().into()
	}
}

sp_api::mock_impl_runtime_apis! {
	impl BlockProductionLogApi<Block, Member> for TestApi {
		fn get_author(_slot: Slot) -> Option<Member> {
			self.author
		}
	}
}

#[test]
fn provides_author_when_runtime_api_returns_one() {
	let mock_api = TestApi { author: Some(102) };

	let provider = BlockAuthorInherentProvider::<Author>::new(
		&mock_api,
		<Block as BlockT>::Hash::default(),
		Slot::from(42),
	)
	.expect("Should not fail");

	assert_eq!(provider.author, mock_api.author.map(Author::from));
}

#[test]
fn skips_providing_author_when_runtime_api_returns_none() {
	let mock_api = TestApi { author: None };

	let provider = BlockAuthorInherentProvider::<Author>::new(
		&mock_api,
		<Block as BlockT>::Hash::default(),
		Slot::from(42),
	)
	.expect("Should not fail");

	assert_eq!(provider.author, None);
}
