//! # Partner Chain Token Bridge
//!
//! This crate defines common primitive types, the inherent data provider and data source API
//! for the token bridge feature of Partner Chains Toolkit.
//!
//! ## Overview
//!
//! The bridge feature of the Partner Chains Toolkit allows sending the native token from
//! Cardano to its Partner Chain in a trustless manner, making use of direct observation by
//! the Partner Chain nodes of the Cardano ledger.
//!
//! Each token *transfer* is made by creating a UTXO on a specific *illiquid circulating
//! supply* (ICS) address in which the tokens are locked so they can be released on the
//! Partner Chain. The feature distinguishes two types of a transfer:
//!
//! 1. *User transfers*, which are sent by ordinary users and addressed to a recipient that
//!    is indicated in the datum attached to the transfer's UTXO.
//! 2. *Reserve transfers*, which are sent as part of the Partner Chain's operation, and
//!    allow the chain to gradually move its token reserve (ie. the reserve of tokens used
//!    to pay block producer rewards) from Cardano to its own ledger.
//!
//! Newly made transfers are picked up by the observability layer once their blocks become
//! stable, and are made available to the runtime as inherent data. The bridge pallet in turn
//! makes this data available to the Partner Chains ledger by passing it to the transfer handler,
//! which is left for each Partner Chain's builders to implement according to their particular
//! requirements.
//!
//! ## Usage
//!
//! ### Prerequisites
//!
//! #### Pallet and runtime API
//!
//! Consult the documentation of `pallet_partner_chains_bridge` for instructions on how to add
//! the bridge pallet to the runtime and implement runtime APIs. The inherent data provider
//! defined in this crate requires [TokenBridgeIDPRuntimeApi] to be implemented in the runtime.
//!
//! #### Data source
//!
//! A data source implementing [TokenBridgeDataSource] is required to be present in the node.
//! The `partner_chains_db_sync_data_sources` crate provides a production ready Db-Sync-based
//! implementation, and a mocked implementation is provided by `partner-chains-mock-data-sources`.
//! See the documentation of those crates for instructions on how to add these data sources to
//! your node.
//!
//! #### Recipient type
//!
//! All user transfers sent through the bridge are addressed to some recipient identified by a
//! chain-specific type, usually an address or public key. Because the toolkit makes no assumptions
//! about the ledger structure, this type must be provided in various places as a type parameter.
//! This type can be arbitrary, as long as it conforms to the trait bounds required by the
//! inherent data provider, data source and the pallet. For simple Substrate chains, the account ID
//! type used by their ledgers is a good choice.
//!
//! ### Adding the inherent data provider
//!
//! Include [TokenBridgeInherentDataProvider] in the list of `InherentDataProviders` of your
//! implementation of [CreateInherentDataProviders]:
//!
//! ```rust
//! use sp_partner_chains_bridge::TokenBridgeInherentDataProvider;
//! struct AccountId;
//! type InherentDataProviders = (
//! 	// sp_timestamp::InherentDataProvider,
//!     // ...
//! 	TokenBridgeInherentDataProvider<AccountId>,
//! );
//! ```
//!
//! The IDP is created the same way when proposing and validating a block:
//! ```rust
//! # use sp_partner_chains_bridge::*;
//! # use sidechain_domain::*;
//! # #[derive(sp_core::Encode)]
//! # struct AccountId;
//! type InherentDataProviders = ( /* other IDPs */ TokenBridgeInherentDataProvider<AccountId>);
//!
//! async fn create_inherent_data_providers<T, Block>(
//!     client: &T,
//!     bridge_data_source: &(impl TokenBridgeDataSource<AccountId> + Send + Sync),
//!     parent_hash: Block::Hash,
//! ) -> Result<InherentDataProviders, Box<dyn std::error::Error + Send + Sync>>
//! where
//!     Block: sp_runtime::traits::Block,
//!     T: sp_api::ProvideRuntimeApi<Block> + Send + Sync,
//!     T::Api: TokenBridgeIDPRuntimeApi<Block> + Send + Sync
//! {
//!     /*
//!      Create other IDPs
//!      */
//!     let mc_hash: McBlockHash = todo!("provided by the MC Hash IDP from `sidechain_mc_hash` crate");
//!
//!     let bridge = TokenBridgeInherentDataProvider::new(
//!     	client,
//!     	parent_hash,
//!     	mc_hash,
//!     	bridge_data_source,
//!     )
//!     .await?;
//!
//!     Ok((/* other IDPs */ bridge))
//! }
//! ```
//!
//! ### Adding to a running chain
//!
//! [TokenBridgeInherentDataProvider] is version-aware and will stay inactive until the pallet
//! is added and fully configured along with [TokenBridgeIDPRuntimeApi]. Thus, the correct order
//! of steps when adding the feature to an already running chain will be:
//!
//! 0. Initialize the bridge smart contracts on Cardano using the offchain provided by the
//!    `partner-chains-cardano-offchain` crate. This step can be performed independently from the
//!    order of other steps.
//! 1. Release a new version of the chain's node, with added data source and inhrent data provider.
//! 2. Distribute the new version and wait until most of the network's nodes have been updated.
//! 3. Perform a runtime upgrade to a runtime version containing the pallet.
//! 4. Complete the configuration of the pallet by setting the correct main chain script and data
//!    checkpoint via an extrinsic. This step requires the governance authority to know the main
//!    chain script values, which it should obtain using the offchain.
//!
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::vec::*;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sidechain_domain::{
	AssetId, AssetName, MainchainAddress, McBlockHash, McBlockNumber, McTxHash, PolicyId,
};
use sp_inherents::*;

#[cfg(feature = "std")]
use {sp_api::ApiExt, sp_api::ProvideRuntimeApi, sp_runtime::traits::Block as BlockT};

/// Smart contract hashes and addresses used by the token bridge on Cardano.
#[derive(
	Default,
	Debug,
	Clone,
	PartialEq,
	Eq,
	TypeInfo,
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	Serialize,
	Deserialize,
)]
pub struct MainChainScripts {
	/// Minting policy ID of the native token
	pub token_policy_id: PolicyId,
	/// Asset name of the native token
	pub token_asset_name: AssetName,
	/// Address of the illiquid supply validator.
	///
	/// All tokens sent to that address are effectively locked and considered "sent" to the Partner Chain.
	pub illiquid_circulation_supply_validator_address: MainchainAddress,
}

impl MainChainScripts {
	/// Return full asset ID fo the bridged token (minting policy ID and asset name)
	pub fn asset_id(&self) -> AssetId {
		AssetId {
			policy_id: self.token_policy_id.clone(),
			asset_name: self.token_asset_name.clone(),
		}
	}
}

#[cfg(feature = "std")]
impl MainChainScripts {
	/// Reads the main chain script values from environment
	///
	/// It expects the following variables to be set:
	/// - `BRIDGE_TOKEN_POLICY_ID`
	/// - `BRIDGE_TOKEN_ASSET_NAME`
	/// - `ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR_ADDRESS`
	pub fn read_from_env() -> Result<Self, envy::Error> {
		#[derive(serde::Serialize, serde::Deserialize)]
		pub struct MainChainScriptsEnvConfig {
			pub bridge_token_policy_id: PolicyId,
			pub bridge_token_asset_name: AssetName,
			pub illiquid_circulation_supply_validator_address: MainchainAddress,
		}

		let MainChainScriptsEnvConfig {
			bridge_token_policy_id,
			bridge_token_asset_name,
			illiquid_circulation_supply_validator_address,
		} = envy::from_env::<MainChainScriptsEnvConfig>()?;

		Ok(Self {
			token_policy_id: bridge_token_policy_id,
			token_asset_name: bridge_token_asset_name,
			illiquid_circulation_supply_validator_address,
		})
	}
}

/// Type containing all information needed to process a single transfer incoming from
/// main chain, corresponding to a single UTXO on Cardano
#[derive(
	Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq, MaxEncodedLen,
)]
pub enum BridgeTransferV1<RecipientAddress> {
	/// Token transfer initiated by a user on Cardano
	UserTransfer {
		/// Amount of tokens tranfered
		token_amount: u64,
		/// Transfer recipient on the Partner Chain
		recipient: RecipientAddress,
	},
	/// Token transfer carrying reserve funds being moved from Cardano to the Partner Chain
	ReserveTransfer {
		/// Amount of tokens tranfered
		token_amount: u64,
	},
	/// Invalid transfer coming from a Transaction on Cardano that does not contain a metadata that can be
	/// correctly interpreted. These transfers can either be ignored and considered lost or recovered
	/// through some custom mechanism.
	InvalidTransfer {
		/// Amount of tokens tranfered
		token_amount: u64,
		/// ID of the UTXO containing an invalid transfer
		tx_hash: sidechain_domain::McTxHash,
	},
}

/// Structure representing all token bridge transfers incoming from Cardano that are to be
/// handled in one Partner Chain block.
#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq)]
pub struct TokenBridgeTransfersV1<RecipientAddress> {
	/// Transfers to be handled in one Partner Chain block
	pub transfers: Vec<BridgeTransferV1<RecipientAddress>>,
	/// Pointer to last data processed, used as an idempotency key by the data layer
	pub data_checkpoint: BridgeDataCheckpoint,
}

/// Inherent identifier used by the Partner Chains token bridge
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"pctokbrg";

/// Error type returned by failing calls of the bridge inherent
#[derive(Debug, Encode, Decode, thiserror::Error, PartialEq)]
pub enum InherentError {
	/// Inherent was not produced when expected
	#[error("Bridge inherent data was present but an inherent was not provided")]
	InherentRequired,
	/// Inherent produced when inherent data not present
	#[error("Bridge inherent produced when no data present")]
	InherentNotExpected,
	/// Inherent produced does not match inherent data
	#[error("Inherent produced does not match inherent data")]
	IncorrectInherent,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

/// Inherent data provider that provides data on token bridge transfers and special token transfers
#[cfg(feature = "std")]
#[derive(Debug)]
pub enum TokenBridgeInherentDataProvider<RecipientAddress> {
	/// Inert IDP. Will not provide inherent data and will never fail.
	Inert,
	/// Version 1
	ActiveV1 {
		/// Token bridge transfer data
		data: TokenBridgeTransfersV1<RecipientAddress>,
	},
}

/// Value specifying the point in time up to which bridge transfers have been processed
///
/// This type is an enum wrapping either a block number or a utxo to handle both a case
/// where all transfers up to a block have been handled and a case where there were more
/// transfers than the limit allows and observability needs to pick up after the last
/// utxo that could be observed
#[derive(
	Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq, MaxEncodedLen,
)]
pub enum BridgeDataCheckpoint {
	/// The last transaction that has been processed
	Tx(McTxHash),
	/// Cardano block up to which data has been processed
	Block(McBlockNumber),
}

/// Interface for data sources that can be used by [TokenBridgeInherentDataProvider]
#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait TokenBridgeDataSource<RecipientAddress>: Send + Sync {
	/// Fetches at most `max_transfers` of token bridge transfers after `data_checkpoint` up to `current_mc_block`
	async fn get_transfers(
		&self,
		main_chain_scripts: MainChainScripts,
		data_checkpoint: BridgeDataCheckpoint,
		max_transfers: u32,
		current_mc_block: McBlockHash,
	) -> Result<
		(Vec<BridgeTransferV1<RecipientAddress>>, BridgeDataCheckpoint),
		Box<dyn std::error::Error + Send + Sync>,
	>;
}

/// Error type returned when creating [TokenBridgeInherentDataProvider] fails
#[cfg(feature = "std")]
#[derive(Debug, thiserror::Error)]
pub enum InherentDataCreationError {
	/// Signals that a runtime API call failed
	#[error("Runtime API call failed: {0}")]
	ApiError(#[from] sp_api::ApiError),
	/// Signals that the data source returned an error
	#[error("Data source call failed: {0}")]
	DataSourceError(Box<dyn std::error::Error + Send + Sync>),
	/// Signals that the current pallet version on the chain is higher than supported by the node's IDP
	#[error("Unsupported pallet version {0} (highest supported version: {1})")]
	UnsupportedPalletVersion(u32, u32),
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl<RecipientAddress: Encode + Send + Sync> sp_inherents::InherentDataProvider
	for TokenBridgeInherentDataProvider<RecipientAddress>
{
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		match self {
			Self::Inert => {},
			Self::ActiveV1 { data } => {
				inherent_data.put_data(INHERENT_IDENTIFIER, data)?;
			},
		}
		Ok(())
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		mut error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		if identifier == &INHERENT_IDENTIFIER {
			let error = InherentError::decode(&mut error).ok()?;
			Some(Err(sp_inherents::Error::Application(Box::from(error))))
		} else {
			None
		}
	}
}

sp_api::decl_runtime_apis! {
	/// Runtime API used by [TokenBridgeInherentDataProvider]
	#[api_version(1)]
	pub trait TokenBridgeIDPRuntimeApi {
		/// Returns the current version of the pallet, 1-based.
		fn get_pallet_version() -> u32;
		/// Returns the currenlty configured main chain scripts
		fn get_main_chain_scripts() -> Option<MainChainScripts>;
		/// Returns the currently configured transfer number limit
		fn get_max_transfers_per_block() -> u32;
		/// Returns last data checkpoint saved in the pallet
		fn get_last_data_checkpoint() -> Option<BridgeDataCheckpoint>;
	}
}

#[cfg(feature = "std")]
impl<RecipientAddress: Encode + Send + Sync> TokenBridgeInherentDataProvider<RecipientAddress> {
	/// Creates new [TokenBridgeInherentDataProvider]
	///
	/// This function is version-aware and will create [TokenBridgeInherentDataProvider] based on
	/// the version reported by the pallet through [TokenBridgeIDPRuntimeApi].
	pub async fn new<Block, T>(
		client: &T,
		parent_hash: Block::Hash,
		current_mc_hash: McBlockHash,
		data_source: &(dyn TokenBridgeDataSource<RecipientAddress> + Send + Sync),
	) -> Result<Self, InherentDataCreationError>
	where
		Block: BlockT,
		T: ProvideRuntimeApi<Block> + Send + Sync,
		T::Api: TokenBridgeIDPRuntimeApi<Block>,
	{
		let api = client.runtime_api();

		let Some(pallet_version) =
			api.api_version::<dyn TokenBridgeIDPRuntimeApi<Block>>(parent_hash)?
		else {
			log::info!(
				"💤 Skipping token bridge transfer observation. Pallet not detected in the runtime."
			);
			return Ok(Self::Inert);
		};

		match pallet_version {
			1 => Self::new_v1(api, parent_hash, current_mc_hash, data_source).await,
			unsupported_version => {
				Err(InherentDataCreationError::UnsupportedPalletVersion(unsupported_version, 1))
			},
		}
	}

	/// Creates new [TokenBridgeInherentDataProvider::ActiveV1]
	pub async fn new_v1<'a, Block, Api>(
		api: sp_api::ApiRef<'a, Api>,
		parent_hash: Block::Hash,
		current_mc_hash: McBlockHash,
		data_source: &dyn TokenBridgeDataSource<RecipientAddress>,
	) -> Result<Self, InherentDataCreationError>
	where
		Block: BlockT,
		Api: TokenBridgeIDPRuntimeApi<Block>,
	{
		let max_transfers = api.get_max_transfers_per_block(parent_hash)?;
		let (Some(last_checkpoint), Some(main_chain_scripts)) =
			(api.get_last_data_checkpoint(parent_hash)?, api.get_main_chain_scripts(parent_hash)?)
		else {
			log::info!("💤 Skipping token bridge transfer observation. Pallet not configured.");
			return Ok(Self::Inert);
		};

		let (transfers, new_checkpoint) = data_source
			.get_transfers(main_chain_scripts, last_checkpoint, max_transfers, current_mc_hash)
			.await
			.map_err(InherentDataCreationError::DataSourceError)?;

		Ok(Self::ActiveV1 {
			data: TokenBridgeTransfersV1 { transfers, data_checkpoint: new_checkpoint },
		})
	}
}
