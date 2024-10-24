use sp_core::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode)]
pub struct CardanoObservabilityConfig {
	pub associations_validator_address: sidechain_domain::MainchainAddress,
	pub associations_policy_id: sidechain_domain::PolicyId,
	pub associations_asset_name: sidechain_domain::AssetName,
}

pub type CardanoMainChainAddress = sidechain_domain::MainchainAddress;

pub type CardanoSyncState = sidechain_domain::UtxoId;

pub type CardanoAddressAssociationInherentData<PartnerChainAddres> =
	crate::inherent_data::AddressAssociationInherentData<
		CardanoMainChainAddress,
		PartnerChainAddres,
		CardanoSyncState,
	>;

pub type CardanoAddressAssociationInherentDataProvider<PartnerChainAddress> =
	crate::inherent_data::AddressAssociationsInherentDataProvider<
		CardanoMainChainAddress,
		PartnerChainAddress,
		CardanoSyncState,
	>;

#[cfg(feature = "std")]
pub mod inherent_data {
	use super::*;
	use crate::AddressAssociationsObservabilityApi;
	use sp_api::ProvideRuntimeApi;
	use sp_runtime::traits::Block as BlockT;

	pub trait CardanoAddressAssociationDataSource<PartnerChainAddress> {
		fn get_address_associations(
			&self,
			config: CardanoObservabilityConfig,
			previous_sync_state: Option<CardanoSyncState>,
			to_mc_hash: sidechain_domain::McBlockHash,
		) -> Result<
			(CardanoSyncState, Vec<(CardanoMainChainAddress, PartnerChainAddress)>),
			Box<dyn std::error::Error + Send + Sync>,
		>;
	}

	impl<PartnerChainAddress> CardanoAddressAssociationInherentDataProvider<PartnerChainAddress> {
		pub fn new<C, Block>(
			data_source: &(dyn CardanoAddressAssociationDataSource<PartnerChainAddress>
			      + Send
			      + Sync),
			client: &C,
			parent_hash: Block::Hash,
			mc_hash: sidechain_domain::McBlockHash,
		) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
		where
			Block: BlockT,
			C: ProvideRuntimeApi<Block>,
			C::Api: AddressAssociationsObservabilityApi<
				Block,
				CardanoObservabilityConfig,
				CardanoSyncState,
			>,
		{
			let api = client.runtime_api();
			let sync_state = api.get_current_sync_state(parent_hash)?;
			let Some(config) = api.get_observability_configuration(parent_hash)? else {
				return Ok(Self::Inactive);
			};
			let (new_sync_state, associations) =
				data_source.get_address_associations(config, sync_state, mc_hash)?;

			Ok(Self::Active { new_sync_state, associations })
		}
	}
}
