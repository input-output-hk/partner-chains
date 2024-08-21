#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub use inherent_provider::*;

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use sidechain_domain::*;
use sp_inherents::*;
use sp_runtime::{scale_info::TypeInfo, traits::Block as BlockT};

#[cfg(test)]
mod tests;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"nattoken";

#[derive(Default, Debug, Clone, PartialEq, Eq, TypeInfo, Encode, Decode, MaxEncodedLen)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MainChainScripts {
	pub native_token_policy: PolicyId,
	pub native_token_asset_name: AssetName,
	pub illiquid_supply_address: MainchainAddress,
}

sp_api::decl_runtime_apis! {
	pub trait NativeTokenManagementApi {
		fn get_main_chain_scripts() -> MainChainScripts;
	}
}

#[derive(Decode, Encode)]
pub struct TokenTransferData {
	pub token_amount: NativeTokenAmount,
}

#[cfg(feature = "std")]
mod inherent_provider {
	use super::*;
	use main_chain_follower_api::{DataSourceError, NativeTokenManagementDataSource};
	use sidechain_mc_hash::get_mc_hash_for_block;
	use sp_api::{ApiError, ProvideRuntimeApi};
	use sp_blockchain::HeaderBackend;
	use std::error::Error;
	use std::sync::Arc;

	pub struct NativeTokenManagementInherentDataProvider {
		pub token_amount: NativeTokenAmount,
	}

	#[derive(thiserror::Error, sp_runtime::RuntimeDebug)]
	pub enum IDPCreationError {
		#[error("Failed to read native token data from data source: {0:?}")]
		DataSourceError(#[from] DataSourceError),
		#[error("Failed to retrieve main chain scripts from the runtime: {0:?}")]
		GetMainChainScriptsError(ApiError),
		#[error("Failed to retrieve previous MC hash: {0:?}")]
		McHashError(Box<dyn Error + Send + Sync>),
	}

	impl NativeTokenManagementInherentDataProvider {
		pub async fn new<Block, C>(
			client: Arc<C>,
			data_source: &(dyn NativeTokenManagementDataSource + Send + Sync),
			mc_hash: McBlockHash,
			parent_hash: <Block as BlockT>::Hash,
		) -> Result<Self, IDPCreationError>
		where
			Block: BlockT,
			C: HeaderBackend<Block>,
			C: ProvideRuntimeApi<Block> + Send + Sync,
			C::Api: NativeTokenManagementApi<Block>,
		{
			let api = client.runtime_api();
			let scripts = api
				.get_main_chain_scripts(parent_hash)
				.map_err(IDPCreationError::GetMainChainScriptsError)?;
			let parent_mc_hash: Option<McBlockHash> =
				get_mc_hash_for_block(client.as_ref(), parent_hash)
					.map_err(IDPCreationError::McHashError)?;
			let token_amount = data_source
				.get_total_native_token_transfer(
					parent_mc_hash,
					mc_hash,
					scripts.native_token_policy,
					scripts.native_token_asset_name,
					scripts.illiquid_supply_address,
				)
				.await?;

			Ok(Self { token_amount })
		}
	}

	#[async_trait::async_trait]
	impl InherentDataProvider for NativeTokenManagementInherentDataProvider {
		async fn provide_inherent_data(
			&self,
			inherent_data: &mut InherentData,
		) -> Result<(), sp_inherents::Error> {
			inherent_data.put_data(
				INHERENT_IDENTIFIER,
				&TokenTransferData { token_amount: self.token_amount.clone() },
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
