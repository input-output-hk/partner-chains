#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use {core::str::FromStr, sp_runtime::traits::Block as BlockT};

#[cfg(feature = "std")]
pub use inherent_provider::*;

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use sidechain_domain::*;
use sp_inherents::*;
use sp_runtime::scale_info::TypeInfo;

#[cfg(test)]
mod tests;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"nattoken";

/// Values identifying on-chain entities involved in the native token management system on Cardano.
#[derive(Default, Debug, Clone, PartialEq, Eq, TypeInfo, Encode, Decode, MaxEncodedLen)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MainChainScripts {
	/// Minting policy ID of the native token
	pub native_token_policy_id: PolicyId,
	/// Asset name of the native token
	pub native_token_asset_name: AssetName,
	/// Address of the illiquid supply validator. All tokens sent to that address are effectively locked
	/// and considered "sent" to the Partner Chain.
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
		fn get_main_chain_scripts() -> Option<MainChainScripts>;
		/// Gets current initializaion status and set it to `true` afterwards. This check is used to
		/// determine whether historical data from the beginning of main chain should be queried.
		fn initialized() -> bool;
	}
}

#[derive(Decode, Encode)]
pub struct TokenTransferData {
	pub token_amount: NativeTokenAmount,
}

#[derive(Encode, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum InherentError {
	#[cfg_attr(feature = "std", error("Inherent missing for token transfer of {0} tokens"))]
	TokenTransferNotHandled(NativeTokenAmount),
	#[cfg_attr(
		feature = "std",
		error("Incorrect token transfer amount: expected {0}, got {1} tokens")
	)]
	IncorrectTokenNumberTransfered(NativeTokenAmount, NativeTokenAmount),
	#[cfg_attr(feature = "std", error("Unexpected transfer of {0} tokens"))]
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
	use sidechain_mc_hash::get_mc_hash_for_block;
	use sp_api::{ApiError, ApiExt, Core, ProvideRuntimeApi};
	use sp_blockchain::HeaderBackend;
	use std::error::Error;
	use std::sync::Arc;

	#[async_trait::async_trait]
	pub trait NativeTokenManagementDataSource {
		/// Retrieves total of native token transfers into the illiquid supply in the range (after_block, to_block]
		async fn get_total_native_token_transfer(
			&self,
			after_block: Option<McBlockHash>,
			to_block: McBlockHash,
			scripts: MainChainScripts,
		) -> Result<NativeTokenAmount, Box<dyn std::error::Error + Send + Sync>>;
	}

	pub struct NativeTokenManagementInherentDataProvider {
		pub token_amount: Option<NativeTokenAmount>,
	}

	#[derive(thiserror::Error, sp_runtime::RuntimeDebug)]
	pub enum IDPCreationError {
		#[error("Failed to read native token data from data source: {0:?}")]
		DataSourceError(Box<dyn Error + Send + Sync>),
		#[error("Failed to call runtime API: {0:?}")]
		ApiError(ApiError),
		#[error("Failed to retrieve previous MC hash: {0:?}")]
		McHashError(Box<dyn Error + Send + Sync>),
	}

	impl From<ApiError> for IDPCreationError {
		fn from(err: ApiError) -> Self {
			Self::ApiError(err)
		}
	}

	impl NativeTokenManagementInherentDataProvider {
		/// Creates inherent data provider only if the pallet is present in the runtime.
		/// Returns zero transfers if not.
		pub async fn new_if_pallet_present<Block, C>(
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
			if client
				.runtime_api()
				.has_api::<dyn NativeTokenManagementApi<Block>>(parent_hash)?
			{
				Self::new(client, data_source, mc_hash, parent_hash).await
			} else {
				Ok(Self { token_amount: None })
			}
		}

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
			let Some(scripts) = api.get_main_chain_scripts(parent_hash)? else {
				return Ok(Self { token_amount: None });
			};
			let parent_mc_hash: Option<McBlockHash> = if api.initialized(parent_hash)? {
				get_mc_hash_for_block(client.as_ref(), parent_hash)
					.map_err(IDPCreationError::McHashError)?
			} else {
				None
			};
			let token_amount = data_source
				.get_total_native_token_transfer(parent_mc_hash, mc_hash, scripts)
				.await
				.map_err(IDPCreationError::DataSourceError)?;

			let token_amount = if token_amount.0 > 0 { Some(token_amount) } else { None };

			Ok(Self { token_amount })
		}
	}

	#[async_trait::async_trait]
	impl InherentDataProvider for NativeTokenManagementInherentDataProvider {
		async fn provide_inherent_data(
			&self,
			inherent_data: &mut InherentData,
		) -> Result<(), sp_inherents::Error> {
			if let Some(token_amount) = self.token_amount {
				inherent_data.put_data(INHERENT_IDENTIFIER, &TokenTransferData { token_amount })
			} else {
				Ok(())
			}
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

	#[cfg(any(test, feature = "mock"))]
	pub mod mock {
		use crate::{MainChainScripts, NativeTokenManagementDataSource};
		use async_trait::async_trait;
		use derive_new::new;
		use sidechain_domain::*;
		use std::collections::HashMap;

		#[derive(new, Default)]
		pub struct MockNativeTokenDataSource {
			transfers: HashMap<(Option<McBlockHash>, McBlockHash), NativeTokenAmount>,
		}

		#[async_trait]
		impl NativeTokenManagementDataSource for MockNativeTokenDataSource {
			async fn get_total_native_token_transfer(
				&self,
				after_block: Option<McBlockHash>,
				to_block: McBlockHash,
				_scripts: MainChainScripts,
			) -> Result<NativeTokenAmount, Box<dyn std::error::Error + Send + Sync>> {
				Ok(self.transfers.get(&(after_block, to_block)).cloned().unwrap_or_default())
			}
		}
	}
}
