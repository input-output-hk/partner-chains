//! Primitives and inherent data provider for the Native Token Management feature
//!
//! # Purpose and context
//!
//! This crate defines shared types used by components that implement the Native Token Management
//! feature of the Partner Chains toolkit, along with an inherent data provider for token transfer
//! data.
//!
//! The Native Token Management feature allows a Partner Chain to keep its token as a native asset
//! on Cardano and have it be transferable to the Partner Chain. This is achieved by the native
//! token being locked at an _illiquid supply_ address on Cardano, and the Partner Chain handling
//! this locking event (a _transfer_) after it has been observed as part of a stable block.
//!
//! The inherent data provider defined in this crate is responsible for providing information about
//! the transfers in form of inherent data, and handling them is the responsibility of the pallet,
//! which allows the Partner Chain builders to define their own transfer handling logic to suit
//! their needs.
//!
//! # Usage
//!
//! ## Prerequisites
//!
//! This features depends on the MC Reference Hash feature to provide Cardano block reference for
//! querying the token transfers. See the documentation of `sidechain_mc_hash` crate for more information.
//!
//! ## Implementing runtime APIs
//!
//! [NativeTokenManagementInherentDataProvider] requires the runtime to implement the
//! [NativeTokenManagementApi] runtime API. This only requires passing relevant values from the pallet:
//! ```rust,ignore
//! impl sp_native_token_management::NativeTokenManagementApi<Block> for Runtime {
//! 	fn get_main_chain_scripts() -> Option<sp_native_token_management::MainChainScripts> {
//! 		NativeTokenManagement::get_main_chain_scripts()
//! 	}
//! 	fn initialized() -> bool {
//! 		NativeTokenManagement::initialized()
//! 	}
//! }
//! ```
//!
//! ## Adding the inherent data provider
//!
//! The inherent data provider requires a data source implementing [NativeTokenManagementDataSource].
//! A Db-Sync implementation is provided by the `partner_chains_db_sync_data_sources` crate.
//!
//! With the data source present, the IDP is straightfoward to create:
//!
//! ```rust
//! use std::sync::Arc;
//! use sp_runtime::traits::Block as BlockT;
//! use sp_native_token_management::*;
//!
//! async fn create_idps<Block: BlockT, C>(
//!     parent_hash: Block::Hash,
//!     client: Arc<C>,
//!     native_token_data_source: &(dyn NativeTokenManagementDataSource + Send + Sync)
//! ) -> Result<(NativeTokenManagementInherentDataProvider /* other IDPs */), Box<dyn std::error::Error + Send + Sync>>
//! where
//!     C: sp_api::ProvideRuntimeApi<Block> + Send + Sync,
//!     C::Api: NativeTokenManagementApi<Block>
//! {
//!     let (mc_hash, previous_mc_hash) = todo!("Should come from the MC Reference Hash feature");
//!
//!     let native_token_idp = NativeTokenManagementInherentDataProvider::new(
//!     	client.clone(),
//!     	native_token_data_source,
//!     	mc_hash,
//!     	previous_mc_hash,
//!     	parent_hash,
//!     )
//!     .await?;
//!     Ok((native_token_idp /* other IDPs */))
//! }
//! ```
//!
//! The same constructor can be used for both proposal and validation of blocks.
//!
//! [NativeTokenManagementApi]: crate::NativeTokenManagementApi
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

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

/// Inherent identifier used by the Native Token Management pallet
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
	/// Reads the main chain script values from local environment variables
	///
	/// It expects the following variables to be set:
	/// - `NATIVE_TOKEN_POLICY_ID`
	/// - `NATIVE_TOKEN_ASSET_NAME`
	/// - `ILLIQUID_SUPPLY_VALIDATOR_ADDRESS`
	pub fn read_from_env() -> Result<Self, envy::Error> {
		#[derive(serde::Serialize, serde::Deserialize)]
		struct MainChainScriptsEnvConfig {
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
	/// Runtime API exposing configuration and initialization status of the Native Token Management pallet
	pub trait NativeTokenManagementApi {
		/// Returns the current main chain scripts configured in the pallet or [None] if they are not set.
		fn get_main_chain_scripts() -> Option<MainChainScripts>;
		/// Gets current initializaion status and set it to `true` afterwards. This check is used to
		/// determine whether historical data from the beginning of main chain should be queried.
		fn initialized() -> bool;
	}
}

/// Data about token transfers in some period of time
#[derive(Decode, Encode)]
pub struct TokenTransferData {
	/// Aggregate number of tokens transfered
	pub token_amount: NativeTokenAmount,
}

/// Error type returned by the Native Token Management pallet inherent logic
#[derive(Encode, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum InherentError {
	/// Signals that no inherent was submitted despite new token transfers being observed
	#[cfg_attr(feature = "std", error("Inherent missing for token transfer of {0} tokens"))]
	TokenTransferNotHandled(NativeTokenAmount),
	/// Signals that the inherent registered an incorrect number of tokens transfered
	#[cfg_attr(
		feature = "std",
		error("Incorrect token transfer amount: expected {0}, got {1} tokens")
	)]
	IncorrectTokenNumberTransfered(NativeTokenAmount, NativeTokenAmount),
	/// Signals that an inherent was submitted when no token transfers were observed
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
	use sp_api::{ApiError, ApiExt, ProvideRuntimeApi};
	use std::error::Error;
	use std::sync::Arc;

	/// Interface for a data source serving native token transfer data compatible with [NativeTokenManagementInherentDataProvider].
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

	/// Inherent data provider that provides aggregate number of native token transfers to the illiquid supply on Cardano
	/// in some time range.
	///
	/// This IDP will not provide any inherent data if `token_amount` is [None], but *will* provide data for `Some(0)`.
	pub struct NativeTokenManagementInherentDataProvider {
		/// Aggregate number of tokens transfered
		pub token_amount: Option<NativeTokenAmount>,
	}

	/// Error type returned when creation of [NativeTokenManagementInherentDataProvider] fails
	#[derive(thiserror::Error, sp_runtime::RuntimeDebug)]
	pub enum IDPCreationError {
		/// Signals that the data source used returned an error
		#[error("Failed to read native token data from data source: {0:?}")]
		DataSourceError(Box<dyn Error + Send + Sync>),
		/// Signals that a runtime API call failed
		#[error("Failed to call runtime API: {0:?}")]
		ApiError(ApiError),
	}

	impl From<ApiError> for IDPCreationError {
		fn from(err: ApiError) -> Self {
			Self::ApiError(err)
		}
	}

	impl NativeTokenManagementInherentDataProvider {
		/// Creates a [NativeTokenManagementInherentDataProvider] that will provide as inherent data the aggregate number of
		/// native token transfers into the illiquid supply on Cardano after the block with hash `parent_mc_hash` up to and
		/// including the block with hash `mc_hash`.
		///
		/// This function is runtime-aware and will only create an active [NativeTokenManagementInherentDataProvider] instance
		/// if the pallet is present and configured. Otherwise the returned inherent data provider will be inactive.
		///
		/// # Arguments
		/// - `client`: runtime client exposing the [NativeTokenManagementApi] runtime API
		/// - `data-source`: data source implementing [NativeTokenManagementDataSource]
		/// - `mc_hash`: main chain block hash referenced by the currently produced or validated block
		/// - `parent_mc_hash`: main chain block hash referenced by the parent of the currently producer or validated block.
		///                     This argument should be [None] if the parent block was the genesis block or didn't reference
		///                     any main chain block.
		/// - `parent_hash`: block hash of the parent block of the currently produced or validated block
		pub async fn new<Block, C>(
			client: Arc<C>,
			data_source: &(dyn NativeTokenManagementDataSource + Send + Sync),
			mc_hash: McBlockHash,
			parent_mc_hash: Option<McBlockHash>,
			parent_hash: <Block as BlockT>::Hash,
		) -> Result<Self, IDPCreationError>
		where
			Block: BlockT,
			C: ProvideRuntimeApi<Block> + Send + Sync,
			C::Api: NativeTokenManagementApi<Block>,
		{
			if client
				.runtime_api()
				.has_api::<dyn NativeTokenManagementApi<Block>>(parent_hash)?
			{
				let api = client.runtime_api();
				let Some(scripts) = api.get_main_chain_scripts(parent_hash)? else {
					return Ok(Self { token_amount: None });
				};
				let token_amount = data_source
					.get_total_native_token_transfer(parent_mc_hash, mc_hash, scripts)
					.await
					.map_err(IDPCreationError::DataSourceError)?;

				let token_amount = if token_amount.0 > 0 { Some(token_amount) } else { None };

				Ok(Self { token_amount })
			} else {
				Ok(Self { token_amount: None })
			}
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

	/// Mock implementation of the data source
	#[cfg(any(test, feature = "mock"))]
	pub mod mock {
		use crate::{MainChainScripts, NativeTokenManagementDataSource};
		use async_trait::async_trait;
		use derive_new::new;
		use sidechain_domain::*;
		use std::collections::HashMap;

		/// Mock implementation of [NativeTokenManagementDataSource]
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
