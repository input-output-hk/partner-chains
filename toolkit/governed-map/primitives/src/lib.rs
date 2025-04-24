//! # Governed Map primitives
//!
//! This crate provides shared types and logic of the Governed Map feature, together with its inherent
//! data provider logic.
//!
//! ## Usage
//!
//!    This crate supports operation of `pallet_governed_map`. Consult the pallet's documentation on how to
//!    include it in the runtime
//!
//! ### Adding to the node
//!
//! #### Implementing the runtime API
//!
//! [GovernedMapInherentDataProvider] requires access to the pallet's configuration via a runtime API.
//! Assuming the pallet has been added to the runtime and is named `GovernedMap`, the API should be
//! implemented like this:
//!
//! ```rust,ignore
//! impl sp_governed_map::GovernedMapIDPApi<Block> for Runtime {
//!     fn get_main_chain_scripts() -> Option<MainChainScriptsV1> {
//!         GovernedMap::get_main_chain_scripts()
//!     }
//!     fn get_pallet_version() -> u32 {
//!         GovernedMap::get_version()
//!     }
//! }
//! ```
//!
//! #### Adding the data source
//!
//! [GovernedMapInherentDataProvider] needs a data source implementing the [GovernedMapDataSource] trait.
//! For nodes using Db-Sync, one is provided by the `partner_chains_db_sync_data_sources` crate. Consult
//! its documentation for more information.
//!
//! #### Adding the inherent data provider
//!
//! The [GovernedMapInherentDataProvider] should be added to your IDP stack using [GovernedMapInherentDataProvider::new]
//! for both block proposal and validation, like so:
//! ```rust
//! # use sp_governed_map::*;
//! # use sidechain_domain::*;
//! type InherentDataProviders = (/* other IDPs */ GovernedMapInherentDataProvider);
//! async fn create_idps<T, Block>(
//!     client: &T,
//!     governed_map_data_source: &(impl GovernedMapDataSource + Send + Sync),
//!     parent_hash: Block::Hash,
//! #   // Arguments below should be provided by the MC Hash IDP from `sidechain_mc_hash` crate
//! #   mc_hash: McBlockHash,
//! #   previous_mc_hash: Option<McBlockHash>,
//! ) -> Result<InherentDataProviders, Box<dyn std::error::Error + Send + Sync>>
//! where
//!     Block: sp_runtime::traits::Block,
//!     T: sp_api::ProvideRuntimeApi<Block> + Send + Sync,
//!     T::Api: GovernedMapIDPApi<Block> + Send + Sync
//! {
//!     /*
//!      Create other IDPs
//!      */
//!     let governed_map = GovernedMapInherentDataProvider::new(
//!         client,
//!         parent_hash,
//!         mc_hash,
//!         previous_mc_hash,
//!         governed_map_data_source
//!     )
//!     .await?;
//!     Ok((/* other IDPs */ governed_map))
//! # }
//! ```
//!
//! Note that it requires access to the current and previous referenced Cardano block hash (`mc_hash` and `previous_mc_hash`).
//! These are provided by the Partner Chains Toolkit's MC Hash inherent data provider from the `sidechain_mc_hash` crate.
//!
//! ## Adding to a running chain
//!
//! As with any other feature, if the Governed Map feature is to be added to an already running chain, a strict order
//! of operations is required:
//! 1. The node should be updated according to the steps described above, so that the inherent data provider is present
//!    in the nodes IDP stack for both proposal and verfication.
//! 2. The updated node binary should be distributed to the block producers who should update their nodes.
//! 3. A new runtime version should be released with the pallet added according to its documentation and the runtime
//!    API implemented as described above.
//! 4. The Governed Map main chain scripts should be set through the `set_main_chain_scripts` extrinsic in the pallet.
//!
//! [GovernedMapInherentDataProvider] is version-aware and will stay inactive until the pallet is added and fully configured.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::fmt::Debug;
use alloc::string::String;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sidechain_domain::{byte_string::*, *};
#[cfg(feature = "std")]
use sp_api::*;
use sp_inherents::*;
use sp_runtime::traits::Get;
use sp_runtime::BoundedVec;

#[cfg(any(test, feature = "mock"))]
mod mock;
#[cfg(test)]
mod tests;

/// Inherent identifier used by the Governed Map pallet
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"govrnmap";

/// Cardano identifiers necessary for observation of the Governed Map
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	TypeInfo,
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	Default,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MainChainScriptsV1 {
	/// Cardano address of the Governed Map validator, at which UTXOs containig key-value pairs are located
	pub validator_address: MainchainAddress,
	/// Asset used to mark the UTXOs containing the Governed Map's key-value pairs
	pub asset: AssetId,
}

/// Type describing a change made to a single key-value pair in the Governed Map.
#[derive(Decode, Encode, TypeInfo, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GovernedMapChangeV1 {
	/// Key of the entry being modified
	pub key: String,
	/// New value under `key`
	/// * [None] value indicates deletion
	/// * [Some] value indicates insertion or update
	pub new_value: Option<ByteString>,
}

/// Error type returned when creating or validating the Governed Map inherent
#[derive(Decode, Encode, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum InherentError {
	/// Signals that the inherent was expected but not produced
	#[cfg_attr(feature = "std", error("Inherent missing for Governed Map pallet"))]
	InherentMissing,
	/// Signals that the inherent was produced when not expected
	#[cfg_attr(feature = "std", error("Unexpected inherent for Governed Map pallet"))]
	InherentNotExpected,
	#[cfg_attr(
		feature = "std",
		error("Data in Governed Map pallet inherent differs from inherent data")
	)]
	/// Signals that the inherent produced contains incorrect data
	IncorrectInherent,
	#[cfg_attr(feature = "std", error("Governed Map key {0} exceeds size bounds"))]
	/// Signals that a key in the mapping change list is longer than the configured bound
	KeyExceedsBounds(String),
	#[cfg_attr(feature = "std", error("Governed Map value {1:?} for key {0} exceeds size bounds"))]
	/// Signals that a value in the mapping change list is longer than the configured bound
	ValueExceedsBounds(String, ByteString),
	/// Signals that the number of unregistered changes to the mapping exceeds the configured upper limit
	///
	/// This should not normally occur if the pallet is configured to accept at least as many changes
	/// as the planned number of keys in use, or if the number of keys exceeds this limit but the number
	/// of changes is low enough not to overwhelm a non-stalled chain.
	///
	/// As this error prevents the production of a block, if this error occurs on a live chain, then the
	/// only way of fixing it is to change the mappings on Cardano close enough to the last state
	/// registered in the pallet to bring the change count below the limit.
	#[cfg_attr(feature = "std", error("Number of changes to the Governed Map exceeds the limit"))]
	TooManyChanges,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

/// Handler trait for runtime components which need to react to changes in the Governed Map.
pub trait OnGovernedMappingChange<MaxKeyLength, MaxValueLength>
where
	MaxKeyLength: Get<u32>,
	MaxValueLength: Get<u32>,
{
	/// Processes a change to a single governed mapping.
	fn on_governed_mapping_change(
		key: BoundedString<MaxKeyLength>,
		new_value: Option<BoundedVec<u8, MaxValueLength>>,
		old_value: Option<BoundedVec<u8, MaxValueLength>>,
	);
}

impl<MaxKeyLength, MaxValueLength> OnGovernedMappingChange<MaxKeyLength, MaxValueLength> for ()
where
	MaxKeyLength: Get<u32>,
	MaxValueLength: Get<u32>,
{
	fn on_governed_mapping_change(
		_key: BoundedString<MaxKeyLength>,
		_new_value: Option<BoundedVec<u8, MaxValueLength>>,
		_old_value: Option<BoundedVec<u8, MaxValueLength>>,
	) {
	}
}

macro_rules! impl_tuple_on_governed_mapping_change {
    ($first_type:ident, $($type:ident),+) => {
		impl<MaxKeyLength, MaxValueLength, $first_type, $($type),+>
			OnGovernedMappingChange<MaxKeyLength, MaxValueLength>
		for ($first_type, $($type),+) where
			MaxKeyLength: Get<u32>,
			MaxValueLength: Get<u32>,
			$first_type: OnGovernedMappingChange<MaxKeyLength, MaxValueLength>,
			$($type: OnGovernedMappingChange<MaxKeyLength, MaxValueLength>),+
		{
			fn on_governed_mapping_change(
				key: BoundedString<MaxKeyLength>,
				new_value: Option<BoundedVec<u8, MaxValueLength>>,
				old_value: Option<BoundedVec<u8, MaxValueLength>>,
			) {
				<$first_type as OnGovernedMappingChange<MaxKeyLength, MaxValueLength>>::on_governed_mapping_change(key.clone(), new_value.clone(), old_value.clone());
				$(<$type as OnGovernedMappingChange<MaxKeyLength, MaxValueLength>>::on_governed_mapping_change(key.clone(), new_value.clone(), old_value.clone());)+
			}
		}
    };
}

impl_tuple_on_governed_mapping_change!(A, B);
impl_tuple_on_governed_mapping_change!(A, B, C);
impl_tuple_on_governed_mapping_change!(A, B, C, D);
impl_tuple_on_governed_mapping_change!(A, B, C, D, E);

type ChangesV1 = Vec<GovernedMapChangeV1>;

/// Inherent data provider providing the list of Governed Map changes that occurred since previous observation.
#[cfg(feature = "std")]
#[derive(Debug, PartialEq)]
pub enum GovernedMapInherentDataProvider {
	/// Inactive variant that will not provide any data and will not raise any errors.
	Inert,
	/// Active variant that will provide data.
	ActiveV1 {
		/// List of changes to the Governed Map that occurred since previous observation
		changes: ChangesV1,
	},
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for GovernedMapInherentDataProvider {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut sp_inherents::InherentData,
	) -> Result<(), sp_inherents::Error> {
		match self {
			Self::ActiveV1 { changes } if !changes.is_empty() => {
				inherent_data.put_data(INHERENT_IDENTIFIER, &changes)?;
			},
			_ => {},
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

/// Cardano observability data source API used by [GovernedMapInherentDataProvider].
#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait GovernedMapDataSource {
	/// Queries all changes that occured in the mappings of the Governed Map on Cardano in the given range of blocks.
	///
	/// # Arguments:
	/// - `since_mc_block`: lower bound (exclusive). If [None], the data source should return all changes since the genesis block.
	/// - `up_to_block`: upper bound (inclusive).
	///
	/// # Return value:
	/// Items in the returned vector should be key-value pairs representing changes to the Governed Map. Inserts and updates should
	/// be represented as a [Some] containing the new value, while deletions should be indicated by a [None]. The vector should be
	/// ordered from the oldest change to the newest.
	async fn get_mapping_changes(
		&self,
		since_mc_block: Option<McBlockHash>,
		up_to_mc_block: McBlockHash,
		main_chain_scripts: MainChainScriptsV1,
	) -> Result<Vec<(String, Option<ByteString>)>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Error type returned when creation of [GovernedMapInherentDataProvider] fails.
#[cfg(feature = "std")]
#[derive(Debug, thiserror::Error)]
pub enum InherentProviderCreationError {
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
impl GovernedMapInherentDataProvider {
	/// Creates a new [GovernedMapInherentDataProvider] with reference to the passed Cardano block hash.
	///
	/// This function is version aware and will return:
	/// - an inert IDP if the pallet is not present in the runtime
	/// - an inert IDP if the pallet does not have main chain scripts configured yet
	/// - an IDP compatible with the pallet version signalled through the runtime API
	///
	/// Parameters:
	/// - `client`: runtime client capable of providing [GovernedMapIDPApi] runtime API
	/// - `parent_hash`: parent hash of the current block
	/// - `mc_hash`: Cardano block hash referenced by the current block
	/// - `data_source`: data source implementing [GovernedMapDataSource]
	pub async fn new<T, Block>(
		client: &T,
		parent_hash: Block::Hash,
		mc_hash: McBlockHash,
		parent_mc_hash: Option<McBlockHash>,
		data_source: &(dyn GovernedMapDataSource + Send + Sync),
	) -> Result<Self, InherentProviderCreationError>
	where
		Block: sp_runtime::traits::Block,
		T: ProvideRuntimeApi<Block> + Send + Sync,
		T::Api: GovernedMapIDPApi<Block>,
	{
		let api = client.runtime_api();
		if !api.has_api::<dyn GovernedMapIDPApi<Block>>(parent_hash)? {
			log::info!("💤 Skipping Governed Map observation. Pallet not detected in the runtime.");
			return Ok(Self::Inert);
		}

		match api.get_pallet_version(parent_hash)? {
			1 => Self::new_v1(client, parent_hash, mc_hash, parent_mc_hash, data_source).await,
			unsupported_version => {
				Err(InherentProviderCreationError::UnsupportedPalletVersion(unsupported_version, 1))
			},
		}
	}

	async fn new_v1<T, Block>(
		client: &T,
		parent_hash: Block::Hash,
		mc_hash: McBlockHash,
		parent_mc_hash: Option<McBlockHash>,
		data_source: &(dyn GovernedMapDataSource + Send + Sync),
	) -> Result<Self, InherentProviderCreationError>
	where
		Block: sp_runtime::traits::Block,
		T: ProvideRuntimeApi<Block> + Send + Sync,
		T::Api: GovernedMapIDPApi<Block>,
	{
		let api = client.runtime_api();

		let Some(main_chain_script) = api.get_main_chain_scripts(parent_hash)? else {
			log::info!("💤 Skipping Governed Map observation. Main chain scripts not set yet.");
			return Ok(Self::Inert);
		};

		let current_entries = data_source
			.get_mapping_changes(parent_mc_hash, mc_hash, main_chain_script)
			.await
			.map_err(InherentProviderCreationError::DataSourceError)?;

		let mut changes: ChangesV1 = ChangesV1::new();

		for (key, value) in current_entries.into_iter() {
			let key = key.into();
			let change = match value {
				None => GovernedMapChangeV1 { key, new_value: None },
				Some(value) => GovernedMapChangeV1 { key, new_value: Some(value.into()) },
			};
			changes.push(change);
		}

		changes.sort();

		Ok(Self::ActiveV1 { changes })
	}
}

sp_api::decl_runtime_apis! {
	/// Runtime API exposing data required for the [GovernedMapInherentDataProvider] to operate.
	#[api_version(1)]
	pub trait GovernedMapIDPApi
	{
		/// Returns the main chain scripts currently set in the pallet or [None] otherwise
		fn get_main_chain_scripts() -> Option<MainChainScriptsV1>;
		/// Returns the current version of the pallet, 1-based.
		fn get_pallet_version() -> u32;
	}
}
