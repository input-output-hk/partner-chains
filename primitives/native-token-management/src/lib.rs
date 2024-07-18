#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub use inherent_provider::*;

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use sidechain_domain::*;
use sp_inherents::*;
use sp_runtime::{
	scale_info::TypeInfo,
	traits::{Block as BlockT, Header, Zero},
};

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"ntreserv";

#[derive(Default, Debug, Clone, PartialEq, Eq, TypeInfo, Encode, Decode, MaxEncodedLen)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MainChainScripts {
	pub native_token_policy: PolicyId,
	pub illiquid_supply_address: MainchainAddress,
}

sp_api::decl_runtime_apis! {
	pub trait NativeTokenManagementApi {
		fn get_main_chain_scripts() -> MainChainScripts;
	}
}

#[derive(Decode, Encode)]
pub struct TokenReleaseData<TokenAmount> {
	pub token_amount: TokenAmount,
}

#[cfg(feature = "std")]
mod inherent_provider {
	use super::*;
	use main_chain_follower_api::{DataSourceError, NativeTokenManagementDataSource};
	use sidechain_mc_hash::McHashInherentDigest;
	use sp_api::{ApiError, ProvideRuntimeApi};
	use sp_blockchain::HeaderBackend;
	use std::{iter::Sum, sync::Arc};

	pub struct NativeTokenManagementInherentDataProvider<TokenAmount> {
		token_amount: TokenAmount,
	}

	#[derive(Debug)]
	pub enum IDPCreationError {
		DataSourceError(DataSourceError),
		RuntimeApiError(ApiError),
	}

	impl From<ApiError> for IDPCreationError {
		fn from(err: ApiError) -> Self {
			Self::RuntimeApiError(err)
		}
	}

	impl From<DataSourceError> for IDPCreationError {
		fn from(err: DataSourceError) -> Self {
			Self::DataSourceError(err)
		}
	}

	impl<TokenAmount> NativeTokenManagementInherentDataProvider<TokenAmount> {
		pub async fn new<Block, C>(
			client: Arc<C>,
			data_source: &(dyn NativeTokenManagementDataSource + Send + Sync),
			mc_hash: McBlockHash,
			parent_hash: <Block as BlockT>::Hash,
		) -> Result<Self, IDPCreationError>
		where
			TokenAmount: From<u64> + Sum<TokenAmount>,
			Block: BlockT,
			C: HeaderBackend<Block>,
			C: ProvideRuntimeApi<Block> + Send + Sync,
			C::Api: NativeTokenManagementApi<Block>,
		{
			let api = client.runtime_api();
			let scripts = api.get_main_chain_scripts(parent_hash)?;
			let parent_header = client.header(parent_hash).unwrap().unwrap();
			let parent_mc_hash: Option<McBlockHash> = if !parent_header.number().is_zero() {
				Some(McHashInherentDigest::value_from_digest(&parent_header.digest().logs).unwrap())
			} else {
				None
			};
			let release = data_source
				.get_token_release_events(
					parent_mc_hash,
					mc_hash,
					scripts.native_token_policy,
					scripts.illiquid_supply_address,
				)
				.await?;
			let token_amount = release.map_or(0, |release| release.token_amount).into();
			Ok(Self { token_amount })
		}
	}

	#[async_trait::async_trait]
	impl<TokenAmount> InherentDataProvider for NativeTokenManagementInherentDataProvider<TokenAmount>
	where
		TokenAmount: Encode + Clone + Send + Sync,
	{
		async fn provide_inherent_data(
			&self,
			inherent_data: &mut InherentData,
		) -> Result<(), sp_inherents::Error> {
			inherent_data.put_data(
				INHERENT_IDENTIFIER,
				&TokenReleaseData { token_amount: self.token_amount.clone() },
			)
		}

		async fn try_handle_error(
			&self,
			identifier: &InherentIdentifier,
			_error: &[u8],
		) -> Option<Result<(), sp_inherents::Error>> {
			if *identifier == INHERENT_IDENTIFIER {
				panic!("BUG: {:?} inherent shouldn't return any errors", INHERENT_IDENTIFIER)
			} else {
				None
			}
		}
	}
}
