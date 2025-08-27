//! Primitives and inherent data provider for the token bridge feature of Partner Chains Toolkit.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::vec::*;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sidechain_domain::{AssetName, MainchainAddress, McBlockHash, PolicyId, UtxoId};
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
	pub illiquid_supply_validator_address: MainchainAddress,
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
	/// Invalid transfer coming from a UTXO on Cardano that does not contain a datum that can be
	/// correctly interpreted. These transfers can either be ignored and considered lost or recovered
	/// through some custom mechanism.
	InvalidTransfer {
		/// Amount of tokens tranfered
		token_amount: u64,
		/// ID of the UTXO containing an invalid transfer
		utxo_id: sidechain_domain::UtxoId,
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

/// Pointer to last data processed
#[derive(
	Default,
	Clone,
	Debug,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	PartialEq,
	Eq,
	MaxEncodedLen,
)]
pub struct BridgeDataCheckpoint(pub UtxoId);

/// Interface for data sources that can be used by [TokenBridgeInherentDataProvider]
#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait TokenBridgeDataSource<RecipientAddress> {
	/// Fetches at most `max_transfers` of token bridge transfers after `data_checkpoint` up to `current_mc_block`
	async fn get_transfers(
		&self,
		data_checkpoint: Option<BridgeDataCheckpoint>,
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
		data_source: &dyn TokenBridgeDataSource<RecipientAddress>,
	) -> Result<Self, InherentDataCreationError>
	where
		Block: BlockT,
		T: ProvideRuntimeApi<Block> + Send + Sync,
		T::Api: TokenBridgeIDPRuntimeApi<Block>,
	{
		let api = client.runtime_api();

		if !api.has_api::<dyn TokenBridgeIDPRuntimeApi<Block>>(parent_hash)? {
			log::info!(
				"💤 Skipping token bridge transfer observation. Pallet not detected in the runtime."
			);
			return Ok(Self::Inert);
		}

		match api.get_pallet_version(parent_hash)? {
			1 => Self::new_v1(client, parent_hash, current_mc_hash, data_source).await,
			unsupported_version => {
				Err(InherentDataCreationError::UnsupportedPalletVersion(unsupported_version, 1))
			},
		}
	}

	/// Creates new [TokenBridgeInherentDataProvider::ActiveV1]
	pub async fn new_v1<Block, T>(
		client: &T,
		parent_hash: Block::Hash,
		current_mc_hash: McBlockHash,
		data_source: &dyn TokenBridgeDataSource<RecipientAddress>,
	) -> Result<Self, InherentDataCreationError>
	where
		Block: BlockT,
		T: ProvideRuntimeApi<Block> + Send + Sync,
		T::Api: TokenBridgeIDPRuntimeApi<Block>,
	{
		let api = client.runtime_api();
		let max_transfers = api.get_max_transfers_per_block(parent_hash)?;
		let last_checkpoint = api.get_last_data_checkpoint(parent_hash)?;

		let (transfers, new_checkpoint) =
			(data_source.get_transfers(last_checkpoint, max_transfers, current_mc_hash).await)
				.map_err(InherentDataCreationError::DataSourceError)?;

		Ok(Self::ActiveV1 {
			data: TokenBridgeTransfersV1 { transfers, data_checkpoint: new_checkpoint },
		})
	}
}
