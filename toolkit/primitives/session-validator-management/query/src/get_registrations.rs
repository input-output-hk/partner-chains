use crate::*;
use authority_selection_inherents::filter_invalid_candidates::RegistrationDataError;
use authority_selection_inherents::filter_invalid_candidates::{
	CandidateValidationApi, StakeError,
};
use sidechain_domain::{
	CandidateRegistrations, MainchainAddress, McEpochNumber, RegistrationData, StakeDelegation,
};
use sp_core::bytes::to_hex;
use sp_sidechain::GetGenesisUtxo;

impl<
		C,
		Block,
		SessionKeys: parity_scale_codec::Decode + Send + Sync + 'static,
		CrossChainPublic,
	> SessionValidatorManagementQuery<C, Block, SessionKeys, CrossChainPublic>
where
	Block: BlockT,
	C: HeaderBackend<Block>,
	C: 'static + ProvideRuntimeApi<Block> + Send + Sync,
	C::Api: GetGenesisUtxo<Block>,
	C::Api: CandidateValidationApi<Block>,
{
	async fn registrations_to_rpc_response(
		&self,
		candidate_registrations: Vec<CandidateRegistrations>,
	) -> Result<GetRegistrationsResponseMap, String> {
		let api = self.client.runtime_api();
		let best_block = self.client.info().best_hash;

		let registration_data_validation =
			|pub_key: &StakePoolPublicKey, registration_data: &RegistrationData| {
				api.validate_registered_candidate_data(best_block, pub_key, registration_data)
			};
		let stake_validation = |stake_delegation: &Option<StakeDelegation>| {
			api.validate_stake(best_block, *stake_delegation)
		};
		get_registrations_response_map(
			candidate_registrations,
			registration_data_validation,
			stake_validation,
		)
		.map_err(|err| format!("{err:?}"))
	}

	pub(crate) async fn candidates_registrations_for_epoch(
		&self,
		mc_epoch_number: McEpochNumber,
		committee_candidate_address: MainchainAddress,
	) -> Result<GetRegistrationsResponseMap, String> {
		let candidates = get_candidates_for_epoch(
			mc_epoch_number,
			self.candidate_data_source.as_ref(),
			committee_candidate_address,
		)
		.await?;
		self.registrations_to_rpc_response(candidates).await
	}
}

/// Creates a map that maps a Candidate's Mainchain Public Key to its Registration Information
fn get_registrations_response_map(
	candidates: Vec<CandidateRegistrations>,
	validate_registration_data: impl Fn(
		&StakePoolPublicKey,
		&RegistrationData,
	) -> Result<Option<RegistrationDataError>, sp_api::ApiError>,
	validate_stake: impl Fn(&Option<StakeDelegation>) -> Result<Option<StakeError>, sp_api::ApiError>,
) -> Result<GetRegistrationsResponseMap, sp_api::ApiError> {
	let mut map = GetRegistrationsResponseMap::new();

	for candidate in candidates {
		let mainchain_pub_key = candidate.mainchain_pub_key().clone();

		let mut registration_entries: Vec<CandidateRegistrationEntry> = candidate
			.registrations
			.iter()
			.map(|registration_data| {
				let registration_data_validation_result =
					validate_registration_data(&mainchain_pub_key, registration_data)?;
				Ok::<CandidateRegistrationEntry, sp_api::ApiError>(CandidateRegistrationEntry::new(
					registration_data.clone(),
					mainchain_pub_key.clone(),
					candidate.stake_delegation,
					registration_data_validation_result,
				))
			})
			.collect::<Result<Vec<_>, _>>()?;

		registration_entries.sort_by_key(|entry| entry.utxo.ordering_key());
		let latest_valid_or_zero = registration_entries
			.iter()
			.rposition(|registration| registration.is_valid)
			.unwrap_or(0);

		registration_entries.drain(..latest_valid_or_zero);
		if let Some(err) = validate_stake(&candidate.stake_delegation)? {
			if let Some(first) = registration_entries.first_mut() {
				if first.is_valid {
					first.is_valid = false;
					first.invalid_reasons = Some(err.into());
				}
			}
		}

		map.insert(to_hex(&mainchain_pub_key.0, false), registration_entries);
	}

	Ok(map)
}

pub(crate) async fn get_candidates_for_epoch(
	mainchain_epoch: McEpochNumber,
	candidate_data_source: &(dyn AuthoritySelectionDataSource + Send + Sync),
	committee_candidate_address: MainchainAddress,
) -> Result<Vec<CandidateRegistrations>, String> {
	candidate_data_source
		.get_candidates(mainchain_epoch, committee_candidate_address)
		.await
		.map_err(|err| format!("{err:?}"))
}
