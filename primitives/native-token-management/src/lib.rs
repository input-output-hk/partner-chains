#![cfg_attr(not(feature = "std"), no_std)]

use core::str::FromStr;

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
	pub native_token_policy_id: PolicyId,
	pub native_token_asset_name: AssetName,
	pub illiquid_supply_validator_address: MainchainAddress,
}

#[cfg(feature = "std")]
impl MainChainScripts {
	pub fn read_from_env() -> Result<Self, envy::Error> {
		#[derive(serde::Serialize, serde::Deserialize)]
		pub struct MainChainScriptsEnvConfig {
			pub native_token_policy_id: PolicyId,
			pub native_token_asset_name: AssetName,
			pub illiquid_supply_validator_address: String,
		}

		let MainChainScriptsEnvConfig {
			native_token_policy_id,
			native_token_asset_name,
			illiquid_supply_validator_address,
		} = envy::from_env()?;
		let illiquid_supply_validator_address =
			FromStr::from_str(&illiquid_supply_validator_address).map_err(|err| {
				envy::Error::Custom(format!("Incorrect main chain address: {}", err))
			})?;
		Ok(Self {
			native_token_policy_id,
			native_token_asset_name,
			illiquid_supply_validator_address,
		})
	}
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

#[derive(Encode, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum InherentError {
	#[cfg_attr(feature = "std", error("Inherent missing for token transfer of {}", 0.0))]
	TokenTransferNotHandled(NativeTokenAmount),
	#[cfg_attr(
		feature = "std",
		error("Incorrect token transfer amount: expected {}, got {}", 0.0, 1.0)
	)]
	IncorrectTokenNumberTransfered(NativeTokenAmount, NativeTokenAmount),
	#[cfg_attr(feature = "std", error("Unexpected transfer of {} tokens", 0.0))]
	UnexpectedTokenTransferInherent(NativeTokenAmount),
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
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
					scripts.native_token_policy_id,
					scripts.native_token_asset_name,
					scripts.illiquid_supply_validator_address,
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
			mut error: &[u8],
		) -> Option<Result<(), sp_inherents::Error>> {
			if *identifier != INHERENT_IDENTIFIER {
				return None;
			}

			let error = InherentError::decode(&mut error).ok()?;

			Some(Err(sp_inherents::Error::Application(Box::from(error))))
		}
	}
}
