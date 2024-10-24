use crate::InherentError;
use parity_scale_codec::Decode;
use parity_scale_codec::Encode;
pub use sp_inherents::InherentData;
use sp_inherents::InherentIdentifier;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"adrassoc";

#[derive(Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub struct AddressAssociationInherentData<MainChainAddress, PartnerChainAddress, SyncState> {
	pub associations: sp_runtime::Vec<(MainChainAddress, PartnerChainAddress)>,
	pub new_sync_state: SyncState,
}

impl<MainChainAddress: Decode, PartnerChainAddress: Decode, SyncState: Decode>
	AddressAssociationInherentData<MainChainAddress, PartnerChainAddress, SyncState>
{
	pub fn from_inherent_data(
		inherent_data: &sp_inherents::InherentData,
	) -> Result<Option<Self>, sp_inherents::Error> {
		inherent_data.get_data::<Self>(&INHERENT_IDENTIFIER)
	}
}

#[cfg(feature = "std")]
#[derive(Debug)]
pub enum AddressAssociationsInherentDataProvider<MainChainAddress, PartnerChainAddress, SyncState> {
	Inactive,
	Active {
		///
		associations: Vec<(MainChainAddress, PartnerChainAddress)>,
		new_sync_state: SyncState,
	},
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl<MainChainAddress, PartnerChainAddress, SyncState> sp_inherents::InherentDataProvider
	for AddressAssociationsInherentDataProvider<MainChainAddress, PartnerChainAddress, SyncState>
where
	MainChainAddress: Send + Sync + Encode + Clone,
	PartnerChainAddress: Send + Sync + Encode + Clone,
	SyncState: Send + Sync + Encode + Clone,
{
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut sp_inherents::InherentData,
	) -> Result<(), sp_inherents::Error> {
		if let Self::Active { associations, new_sync_state } = self {
			let data = AddressAssociationInherentData {
				associations: associations.clone(),
				new_sync_state: new_sync_state.clone(),
			};
			inherent_data.put_data(INHERENT_IDENTIFIER, &data)
		} else {
			Ok(())
		}
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		mut error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		if *identifier == INHERENT_IDENTIFIER {
			let error = InherentError::decode(&mut error).ok()?;
			Some(Err(sp_inherents::Error::Application(Box::from(error))))
		} else {
			None
		}
	}
}
