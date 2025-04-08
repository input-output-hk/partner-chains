#[cfg(feature = "std")]
use crate::authority_selection_inputs::AuthoritySelectionDataSource;
use crate::authority_selection_inputs::AuthoritySelectionInputs;
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "std")]
use {
	crate::authority_selection_inputs::AuthoritySelectionInputsCreationError,
	sidechain_domain::mainchain_epoch::{MainchainEpochConfig, MainchainEpochDerivation},
	sidechain_domain::*,
	sidechain_slots::ScSlotConfig,
	sp_api::ProvideRuntimeApi,
	sp_consensus_slots::Slot,
	sp_inherents::{InherentData, InherentIdentifier},
	sp_runtime::traits::Block as BlockT,
	sp_session_validator_management::CommitteeMember as CommitteeMemberT,
	sp_session_validator_management::{
		InherentError, MainChainScripts, SessionValidatorManagementApi, INHERENT_IDENTIFIER,
	},
};

/// The type of the inherent.
pub type InherentType = AuthoritySelectionInputs;

#[derive(Clone, Debug, Encode, Decode)]
pub struct AriadneInherentDataProvider {
	pub data: Option<AuthoritySelectionInputs>,
}

#[cfg(feature = "std")]
impl AriadneInherentDataProvider {
	pub async fn from_mc_data(
		candidate_data_source: &(dyn AuthoritySelectionDataSource + Send + Sync),
		for_epoch: McEpochNumber,
		scripts: MainChainScripts,
	) -> Result<Self, InherentProviderCreationError> {
		Ok(Self {
			data: Some(
				AuthoritySelectionInputs::from_mc_data(candidate_data_source, for_epoch, scripts)
					.await?,
			),
		})
	}

	pub async fn new<Block, CommitteeMember, T>(
		client: &T,
		sc_slot_config: &ScSlotConfig,
		mc_epoch_config: &MainchainEpochConfig,
		parent_hash: <Block as BlockT>::Hash,
		slot: Slot,
		candidate_data_source: &(dyn AuthoritySelectionDataSource + Send + Sync),
		mc_reference_epoch: McEpochNumber,
	) -> Result<Self, InherentProviderCreationError>
	where
		CommitteeMember: CommitteeMemberT + Decode + Encode,
		CommitteeMember::AuthorityKeys: Decode + Encode,
		CommitteeMember::AuthorityId: Decode + Encode,
		Block: BlockT,
		T: ProvideRuntimeApi<Block> + Send + Sync,
		T::Api: SessionValidatorManagementApi<
			Block,
			CommitteeMember,
			AuthoritySelectionInputs,
			ScEpochNumber,
		>,
	{
		let for_mc_epoch = mc_epoch_for_next_ariadne_cidp(
			client,
			sc_slot_config,
			mc_epoch_config,
			parent_hash,
			slot,
		)?;

		let data_epoch = candidate_data_source.data_epoch(for_mc_epoch).await?;
		// We could accept mc_reference at last slot of data_epoch, but calculations are much easier like that.
		// Additionally, in current implementation, the inequality below is always true, thus there is no need to make it more accurate.
		let scripts = client.runtime_api().get_main_chain_scripts(parent_hash)?;
		if data_epoch < mc_reference_epoch {
			Ok(AriadneInherentDataProvider::from_mc_data(
				candidate_data_source,
				for_mc_epoch,
				scripts,
			)
			.await?)
		} else {
			Ok(AriadneInherentDataProvider { data: None })
		}
	}
}

#[cfg(feature = "std")]
#[derive(Debug, thiserror::Error)]
pub enum InherentProviderCreationError {
	#[error("Slot represents a timestamp bigger than of u64::MAX")]
	SlotTooBig,
	#[error("Couldn't convert timestamp to main chain epoch: {0}")]
	McEpochDerivationError(#[from] sidechain_domain::mainchain_epoch::EpochDerivationError),
	#[error("Runtime API call failed: {0}")]
	ApiError(#[from] sp_api::ApiError),
	#[error("Failed to create authority selection inputs: {0}")]
	InputsCreationError(#[from] AuthoritySelectionInputsCreationError),
	#[error("Data source call failed: {0}")]
	DataSourceError(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(feature = "std")]
fn mc_epoch_for_next_ariadne_cidp<Block, CommitteeMember, T>(
	client: &T,
	sc_slot_config: &ScSlotConfig,
	epoch_config: &MainchainEpochConfig,
	parent_hash: <Block as BlockT>::Hash,
	slot: Slot,
) -> Result<McEpochNumber, InherentProviderCreationError>
where
	Block: BlockT,
	CommitteeMember: Decode + Encode + CommitteeMemberT,
	CommitteeMember::AuthorityKeys: Decode + Encode,
	CommitteeMember::AuthorityId: Decode + Encode,
	T: ProvideRuntimeApi<Block> + Send + Sync,
	T::Api: SessionValidatorManagementApi<
		Block,
		CommitteeMember,
		AuthoritySelectionInputs,
		ScEpochNumber,
	>,
{
	let next_unset_epoch = client.runtime_api().get_next_unset_epoch_number(parent_hash)?;

	let for_sc_epoch_number = {
		// A special case for the genesis committee (current epoch 0, next unset epoch 1).
		// The genesis committee epoch is initialized with 0, so in the very first block we need to provide
		// the epoch number based on the current slot number
		if next_unset_epoch == ScEpochNumber(1) {
			sc_slot_config.epoch_number(slot)
		} else {
			next_unset_epoch
		}
	};

	sc_epoch_to_mc_epoch(for_sc_epoch_number, sc_slot_config, epoch_config)
}

#[cfg(feature = "std")]
fn sc_epoch_to_mc_epoch(
	sc_epoch: ScEpochNumber,
	sc_slot_config: &ScSlotConfig,
	epoch_config: &MainchainEpochConfig,
) -> Result<McEpochNumber, InherentProviderCreationError> {
	let timestamp = sc_slot_config
		.epoch_start_time(sc_epoch)
		.ok_or(InherentProviderCreationError::SlotTooBig)?;

	epoch_config
		.timestamp_to_mainchain_epoch(timestamp)
		.map_err(InherentProviderCreationError::McEpochDerivationError)
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for AriadneInherentDataProvider {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		match &self.data {
			None => Ok(()),
			Some(data) => inherent_data.put_data(INHERENT_IDENTIFIER, data),
		}
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		// Dont' process modules from other inherents
		if *identifier != INHERENT_IDENTIFIER {
			return None;
		}

		let mut error = error;
		Some(Err(sp_inherents::Error::Application(Box::from(
			<InherentError as parity_scale_codec::Decode>::decode(&mut error).ok()?,
		))))
	}
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct CommitteeConfig {
	pub min_size: u16,
	pub max_size: u16,
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ariadne_inherent_data_provider::AriadneInherentDataProvider;
	use crate::mock::MockAuthoritySelectionDataSource;
	use crate::runtime_api_mock::*;
	use sidechain_domain::mainchain_epoch::*;
	use sidechain_slots::*;
	use sp_core::offchain::Timestamp;
	use sp_core::H256;
	use SlotDuration;

	#[tokio::test]
	async fn return_empty_ariadne_cidp_if_runtime_requests_too_new_epoch() {
		// This is the epoch number that is too new
		let next_unset_epoch_number = ScEpochNumber(42);
		let mc_reference_epoch = McEpochNumber(1);
		let empty_ariadne_idp = AriadneInherentDataProvider::new(
			&client(next_unset_epoch_number),
			&sc_slot_config(),
			&epoch_config(),
			H256::zero(),
			// This is the slot that will be used to calculate current_epoch_number
			Slot::from(400u64),
			&MockAuthoritySelectionDataSource::default(),
			mc_reference_epoch,
		)
		.await;

		assert!(empty_ariadne_idp.is_ok());
		assert!(empty_ariadne_idp.unwrap().data.is_none());
	}

	#[tokio::test]
	async fn error_if_num_permissioned_candidates_non_zero_and_no_permissioned_candidate_list() {
		use crate::ariadne_inherent_data_provider::InherentProviderCreationError::InputsCreationError;
		use crate::authority_selection_inputs::AuthoritySelectionInputsCreationError::AriadneParametersQuery;

		let next_unset_epoch_number = ScEpochNumber(42);
		let mc_reference_epoch = McEpochNumber(5);
		let ariadne_idp = AriadneInherentDataProvider::new(
			&client(next_unset_epoch_number),
			&sc_slot_config(),
			&epoch_config(),
			H256::zero(),
			// This is the slot that will be used to calculate current_epoch_number
			Slot::from(400u64),
			&MockAuthoritySelectionDataSource::default()
				.with_permissioned_candidates(vec![None, None, None, None, None])
				.with_num_permissioned_candidates(3),
			mc_reference_epoch,
		)
		.await;

		assert!(ariadne_idp.is_err());
		assert!(matches!(
			ariadne_idp.unwrap_err(),
			InputsCreationError(AriadneParametersQuery(_, _, _, _))
		));
	}

	#[tokio::test]
	async fn ok_if_num_permissioned_candidates_zero_and_no_permissioned_candidate_list() {
		let next_unset_epoch_number = ScEpochNumber(42);
		let mc_reference_epoch = McEpochNumber(5);
		let ariadne_idp = AriadneInherentDataProvider::new(
			&client(next_unset_epoch_number),
			&sc_slot_config(),
			&epoch_config(),
			H256::zero(),
			// This is the slot that will be used to calculate current_epoch_number
			Slot::from(400u64),
			&MockAuthoritySelectionDataSource::default()
				.with_permissioned_candidates(vec![None, None, None, None, None])
				.with_num_permissioned_candidates(0),
			mc_reference_epoch,
		)
		.await;

		assert!(ariadne_idp.is_ok());
		assert!(ariadne_idp.unwrap().data.is_some());
	}

	fn sc_slot_config() -> ScSlotConfig {
		ScSlotConfig {
			slots_per_epoch: SlotsPerEpoch(10),
			slot_duration: SlotDuration::from_millis(1000),
		}
	}

	fn client(next_unset_epoch_number: ScEpochNumber) -> TestApi {
		TestApi { next_unset_epoch_number }
	}

	fn epoch_config() -> MainchainEpochConfig {
		MainchainEpochConfig {
			first_epoch_timestamp_millis: Timestamp::from_unix_millis(0),
			first_epoch_number: 0,
			epoch_duration_millis: Duration::from_millis(
				u64::from(sc_slot_config().slots_per_epoch.0)
					* sc_slot_config().slot_duration.as_millis()
					* 10,
			),
			first_slot_number: 0,
			slot_duration_millis: Duration::from_millis(1000),
		}
	}
}
