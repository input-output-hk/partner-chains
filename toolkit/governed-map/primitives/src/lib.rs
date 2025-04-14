#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::fmt::Debug;
use alloc::string::String;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sidechain_domain::{byte_string::*, *};
use sp_inherents::*;
use sp_runtime::traits::Get;
use sp_runtime::BoundedVec;
#[cfg(feature = "std")]
use {sp_api::*, std::collections::BTreeMap};

#[cfg(any(test, feature = "mock"))]
mod mock;
#[cfg(test)]
mod tests;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"govrnmap";

/// Cardano identifiers necessary for observation of the Governed Map
#[derive(Debug, Clone, PartialEq, Eq, TypeInfo, Encode, Decode, MaxEncodedLen, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MainChainScriptsV1 {
	pub validator_address: MainchainAddress,
	pub asset: AssetId,
}

/// Type describing a change made to a single key-value pair in the Governed Map.
#[derive(Decode, Encode, TypeInfo, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GovernedMapChangeV1 {
	pub key: String,
	pub new_value: Option<ByteString>,
}

impl GovernedMapChangeV1 {
	pub fn upsert(key: &str, new_value: &[u8]) -> Self {
		Self { key: key.into(), new_value: Some(new_value.into()) }
	}
	pub fn delete(key: &str) -> Self {
		Self { key: key.into(), new_value: None }
	}
}

/// Error type returned when creating or validating the Governed Map inherent
#[derive(Decode, Encode, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum InherentError {
	#[cfg_attr(feature = "std", error("Inherent missing for Governed Map pallet"))]
	InherentMissing,
	#[cfg_attr(feature = "std", error("Unexpected inherent for Governed Map pallet"))]
	InherentNotExpected,
	#[cfg_attr(
		feature = "std",
		error("Data in Governed Map pallet inherent differs from inherent data")
	)]
	IncorrectInherent,
	#[cfg_attr(feature = "std", error("Governed Map key {0} exceeds size bounds"))]
	KeyExceedsBounds(String),
	#[cfg_attr(feature = "std", error("Governed Map value {1:?} for key {0} exceeds size bounds"))]
	ValueExceedsBounds(String, ByteString),
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

/// Inherent data provider providing the list of Governed Map changes that occured since previous observation.
#[cfg(feature = "std")]
#[derive(Debug, PartialEq)]
pub enum GovernedMapInherentDataProvider {
	/// Inactive variant that will not provide any data and will not raise any errors.
	Inert,
	/// Active variant that will provide data.
	ActiveV1 { changes: ChangesV1 },
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
	/// Returns all key-value mappings stored in the Governed Map on Cardano after execution of `mc_block`.
	async fn get_current_mappings(
		&self,
		mc_block: McBlockHash,
		main_chain_scripts: MainChainScriptsV1,
	) -> Result<BTreeMap<String, ByteString>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Error type returned when creation of [GovernedMapInherentDataProvider] fails.
#[cfg(feature = "std")]
#[derive(Debug, thiserror::Error)]
pub enum InherentProviderCreationError {
	#[error("Runtime API call failed: {0}")]
	ApiError(#[from] sp_api::ApiError),
	#[error("Data source call failed: {0}")]
	DataSourceError(Box<dyn std::error::Error + Send + Sync>),
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
			1 => Self::new_v1(client, parent_hash, mc_hash, data_source).await,
			unsupported_version => {
				Err(InherentProviderCreationError::UnsupportedPalletVersion(unsupported_version, 1))
			},
		}
	}

	async fn new_v1<T, Block>(
		client: &T,
		parent_hash: Block::Hash,
		mc_hash: McBlockHash,
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

		let entries_in_storage = api.get_stored_mappings(parent_hash)?;
		let current_entries = data_source
			.get_current_mappings(mc_hash, main_chain_script)
			.await
			.map_err(InherentProviderCreationError::DataSourceError)?;

		let mut changes: ChangesV1 = ChangesV1::new();

		for (key, value) in current_entries.iter() {
			if entries_in_storage.get(key) != Some(value) {
				changes.push(GovernedMapChangeV1::upsert(&key, &value));
			}
		}

		for key in entries_in_storage.keys() {
			if !current_entries.contains_key(key) {
				changes.push(GovernedMapChangeV1::delete(&key));
			}
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
		/// Returns all key-value mappings currently stored in the pallet
		fn get_stored_mappings() -> BTreeMap<String, ByteString>;
		/// Returns the main chain scripts currently set in the pallet or [None] otherwise
		fn get_main_chain_scripts() -> Option<MainChainScriptsV1>;
		/// Returns the current version of the pallet, 1-based.
		fn get_pallet_version() -> u32;
	}
}
