mod conversion;
mod mock;
mod query_mock;
mod runtime_api_mock;

use super::SessionValidatorManagementQuery;
use super::types::{CommitteeMember, GetCommitteeResponse};
use super::*;
use crate::tests::query_mock::TestApi;
use authority_selection_inherents::mock::MockAuthoritySelectionDataSource;
use mock::*;
use sidechain_domain::*;
use sp_core::bytes::to_hex;
use sp_core::crypto::Ss58Codec;
use sp_core::{Pair, ecdsa, ed25519};
use sp_runtime::{MultiSigner, traits::IdentifyAccount};
#[allow(deprecated)]
use sp_sidechain::SidechainStatus;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn get_epoch_committee() {
	let client = Arc::new(TestApi {});
	let candidate_data_source = MockAuthoritySelectionDataSource::default();
	let rpc = SessionValidatorManagementQuery::new(client, Arc::new(candidate_data_source));
	let response = rpc.get_epoch_committee(777).unwrap();

	let expected_initial_committee: Vec<_> = runtime_api_mock::committee_for_epoch(777);

	assert_eq!(
		response,
		GetCommitteeResponse {
			sidechain_epoch: 777,
			committee: expected_initial_committee
				.into_iter()
				.map(|key| CommitteeMember::new(key.authority_id().as_ref()))
				.collect()
		}
	)
}

#[tokio::test]
async fn get_epoch_committee_should_not_work_when_committee_is_in_the_future() {
	let client = Arc::new(TestApi {});
	let candidate_data_source = MockAuthoritySelectionDataSource::default();
	let rpc = SessionValidatorManagementQuery::new(client, Arc::new(candidate_data_source));
	let error = rpc.get_epoch_committee(conversion::BEST_EPOCH + 2).unwrap_err();
	assert_eq!(error, format!("Committee is unknown for epoch {}", conversion::BEST_EPOCH + 2));
}

#[tokio::test]
async fn get_epoch_committee_should_not_work_for_epoch_lesser_than_first_epoch() {
	let client = Arc::new(TestApi {});
	let candidate_data_source = MockAuthoritySelectionDataSource::default();
	let rpc = SessionValidatorManagementQuery::new(client, Arc::new(candidate_data_source));
	let error = rpc.get_epoch_committee(conversion::EPOCH_OF_BLOCK_1 - 1).unwrap_err();
	assert_eq!(error, "Epoch 16 is earlier than the Initial Epoch!");
	assert!(rpc.get_epoch_committee(conversion::EPOCH_OF_BLOCK_1).is_ok());
}

#[tokio::test]
async fn get_epoch_committee_should_return_initial_committee_for_genesis_and_first_epoch() {
	let client = Arc::new(TestApi {});
	let candidate_data_source = MockAuthoritySelectionDataSource::default();
	let rpc = SessionValidatorManagementQuery::new(client, Arc::new(candidate_data_source));

	let expected_initial_committee: Vec<_> =
		runtime_api_mock::committee_for_epoch(conversion::GENESIS_EPOCH);
	let expected_initial_committee: Vec<CommitteeMember> = expected_initial_committee
		.into_iter()
		.map(|key| CommitteeMember::new(key.authority_id().as_ref()))
		.collect();

	let GetCommitteeResponse {
		sidechain_epoch: actual_epoch_number,
		committee: actual_initial_committee,
	} = rpc.get_epoch_committee(conversion::GENESIS_EPOCH).unwrap();
	assert_eq!(actual_epoch_number, conversion::GENESIS_EPOCH);
	assert_eq!(actual_initial_committee, expected_initial_committee);

	let GetCommitteeResponse { sidechain_epoch: next_committee_epoch, committee: next_committee } =
		rpc.get_epoch_committee(conversion::EPOCH_OF_BLOCK_1).unwrap();
	assert_eq!(next_committee, expected_initial_committee);
	assert_eq!(next_committee_epoch, conversion::EPOCH_OF_BLOCK_1);
}

#[tokio::test]
async fn get_epoch_committee_should_work_correctly_for_next_epoch() {
	let client = Arc::new(TestApi {});
	let candidate_data_source = MockAuthoritySelectionDataSource::default();
	let rpc = SessionValidatorManagementQuery::new(client, Arc::new(candidate_data_source));
	let epoch = conversion::BEST_EPOCH + 1;
	let response = rpc.get_epoch_committee(epoch).unwrap();

	let expected_committee: Vec<_> = runtime_api_mock::committee_for_epoch(epoch)
		.into_iter()
		.map(|key| CommitteeMember::new(key.authority_id().as_ref()))
		.collect();

	assert_eq!(
		response,
		GetCommitteeResponse { sidechain_epoch: epoch, committee: expected_committee }
	)
}

mod get_registration_tests {
	use super::*;
	use crate::types::PermissionedCandidateData;
	use authority_selection_inherents::filter_invalid_candidates::{
		PermissionedCandidateDataError, RegistrationDataError, StakeError,
	};

	const SEED: [u8; 32] = [7u8; 32];
	const SEED2: [u8; 32] = [8u8; 32];

	fn get_first_candidate_and_first_registration(
		candidates: &[CandidateRegistrations],
	) -> (CandidateRegistrations, RegistrationData) {
		assert!(!candidates.is_empty());
		let first_candidate = candidates[0].clone();
		assert!(first_candidate.registrations().len() == 1);
		let first_registration = first_candidate.registrations()[0].clone();

		(first_candidate, first_registration)
	}

	#[tokio::test]
	async fn should_work() {
		let support_epoch = McEpochNumber(1);
		let candidate_data_source_mock = MockAuthoritySelectionDataSource::default()
			.with_candidates_per_epoch(vec![
				vec![],
				create_candidates(vec![SEED, SEED2], TEST_UTXO_ID),
			]);
		let response = candidate_data_source_mock
			.get_candidates(McEpochNumber(1), MainchainAddress::default())
			.await
			.unwrap();

		let (candidate, registration) = get_first_candidate_and_first_registration(&response);

		let client = Arc::new(TestApi {});
		let api =
			SessionValidatorManagementQuery::new(client, Arc::new(candidate_data_source_mock));

		let registrations = api
			.get_registrations(support_epoch, candidate.mainchain_pub_key().clone())
			.await
			.unwrap();

		let expected_entry = CandidateRegistrationEntry {
			sidechain_pub_key: to_hex(&registration.sidechain_pub_key.0, false),
			sidechain_account_id: MultiSigner::Ecdsa(ecdsa::Public::from(
				<[u8; 33]>::try_from(registration.sidechain_pub_key.0).unwrap(),
			))
			.into_account()
			.to_ss58check(),
			mainchain_pub_key: to_hex(&candidate.mainchain_pub_key().0, false),
			cross_chain_pub_key: to_hex(&registration.cross_chain_pub_key.0, false),
			aura_pub_key: to_hex(&registration.aura_pub_key.0, false),
			grandpa_pub_key: to_hex(&registration.grandpa_pub_key.0, false),
			sidechain_signature: to_hex(&registration.sidechain_signature.0, false),
			mainchain_signature: to_hex(&registration.mainchain_signature.0, false),
			cross_chain_signature: to_hex(&registration.cross_chain_signature.0, false),
			utxo: registration.utxo_info,
			stake_delegation: candidate.stake_delegation.map(|sd| sd.0),
			is_valid: true,
			invalid_reasons: None,
		};

		assert_eq!(registrations, vec![expected_entry])
	}

	#[tokio::test]
	async fn should_not_be_valid_if_signature_is_invalid() {
		test_invalid_registration_data(
			|registration| registration.sidechain_signature = SidechainSignature(vec![7u8; 64]),
			RegistrationError::InvalidRegistrationData(
				RegistrationDataError::InvalidSidechainSignature,
			),
		)
		.await;
	}

	#[tokio::test]
	async fn should_not_be_valid_if_aura_account_key_is_invalid() {
		test_invalid_registration_data(
			|registration| registration.aura_pub_key = AuraPublicKey(vec![7u8; 4]),
			RegistrationError::InvalidRegistrationData(RegistrationDataError::InvalidAuraKey),
		)
		.await;
	}

	#[tokio::test]
	async fn should_not_be_valid_if_grandpa_account_key_is_invalid() {
		test_invalid_registration_data(
			|registration| registration.grandpa_pub_key = GrandpaPublicKey(vec![7u8; 4]),
			RegistrationError::InvalidRegistrationData(RegistrationDataError::InvalidGrandpaKey),
		)
		.await;
	}

	async fn test_invalid_registration_data(
		mut invalidate_registration: impl FnMut(&mut RegistrationData),
		expected_error: RegistrationError,
	) {
		let supported_epoch = McEpochNumber(1);
		let mut candidate_data_source_mock = MockAuthoritySelectionDataSource::default()
			.with_candidates_per_epoch(vec![vec![], create_candidates(vec![SEED], TEST_UTXO_ID)]);
		let candidate = candidate_data_source_mock.candidates[1][0].borrow_mut();
		let mainchain_pub_key_clone = candidate.mainchain_pub_key().clone();
		let registration = candidate.registrations[0].borrow_mut();
		invalidate_registration(registration);

		let client = Arc::new(TestApi {});
		let api =
			SessionValidatorManagementQuery::new(client, Arc::new(candidate_data_source_mock));

		let registrations = api
			.get_registrations(supported_epoch, mainchain_pub_key_clone.clone())
			.await
			.unwrap();

		assert!(!registrations[0].is_valid);
		assert_eq!(registrations[0].invalid_reasons, Some(expected_error));
	}

	#[tokio::test]
	async fn only_last_valid_registration_data_and_newer_invalid_should_be_returned_regardless_of_stake()
	 {
		let stakes = vec![Some(StakeDelegation(5)), None, Some(StakeDelegation(0))];
		for stake in stakes {
			let supported_epoch = McEpochNumber(1);
			let (mainchain_account, _) = ed25519::Pair::generate();
			let (sidechain_account, _) = ecdsa::Pair::generate();
			let stake_pool_public_key = StakePoolPublicKey(mainchain_account.public().0);
			let valid_registration_data = create_valid_registration_data(
				mainchain_account,
				sidechain_account.clone(),
				TEST_UTXO_ID,
			);

			let mut registrations: Vec<RegistrationData> = (0u32..5)
				.map(|block_number| {
					let mut registration = valid_registration_data.clone();
					registration.utxo_info.block_number = McBlockNumber(block_number);
					registration
				})
				.collect();
			registrations.extend(
				(3u32..7)
					.map(|block_number| {
						let mut registration = valid_registration_data.clone();
						registration.utxo_info.block_number = McBlockNumber(block_number);
						registration.utxo_info.tx_index_within_block = McTxIndexInBlock(8);
						registration.sidechain_signature = SidechainSignature(vec![7u8; 64]);
						registration
					})
					.collect::<Vec<_>>(),
			);

			let candidate_data_source_mock = MockAuthoritySelectionDataSource::default()
				.with_candidates_per_epoch(vec![
					vec![],
					vec![CandidateRegistrations {
						stake_pool_public_key: stake_pool_public_key.clone(),
						registrations,
						stake_delegation: stake,
					}],
				]);

			let client = Arc::new(TestApi {});
			let api =
				SessionValidatorManagementQuery::new(client, Arc::new(candidate_data_source_mock));

			let registrations =
				api.get_registrations(supported_epoch, stake_pool_public_key).await.unwrap();

			let block_numbers: Vec<McBlockNumber> = registrations
				.clone()
				.into_iter()
				.map(|candidate| candidate.utxo.block_number)
				.collect();

			assert_eq!(
				block_numbers,
				vec![McBlockNumber(4), McBlockNumber(4), McBlockNumber(5), McBlockNumber(6),]
			);

			assert_eq!(registrations.len(), 4);
			match stake {
				None => {
					assert!(!registrations.first().unwrap().is_valid);
					assert_eq!(
						registrations.first().unwrap().invalid_reasons,
						Some(RegistrationError::StakeError(StakeError::UnknownStake))
					);
				},
				Some(StakeDelegation(0)) => {
					assert!(!registrations.first().unwrap().is_valid);
					assert_eq!(
						registrations.first().unwrap().invalid_reasons,
						Some(RegistrationError::StakeError(StakeError::InvalidStake))
					);
				},
				Some(_) => assert!(registrations.first().unwrap().is_valid),
			}
			for registration in registrations.iter().skip(1) {
				assert!(!registration.is_valid);
				assert!(registration.invalid_reasons.is_some());
				assert!(matches!(
					registration.invalid_reasons,
					Some(RegistrationError::InvalidRegistrationData(_))
				));
			}
		}
	}

	fn valid_permissioned_candidate() -> sidechain_domain::PermissionedCandidateData {
		sidechain_domain::PermissionedCandidateData {
			sidechain_public_key: SidechainPublicKey(vec![1; 33]),
			aura_public_key: AuraPublicKey(vec![2; 32]),
			grandpa_public_key: GrandpaPublicKey(vec![3; 32]),
		}
	}

	#[tokio::test]
	async fn return_correct_ariadne_parameters() {
		let permissioned_candidates = vec![valid_permissioned_candidate()];
		let candidate_registrations = create_candidates(vec![SEED], TEST_UTXO_ID);
		let candidate_data_source_mock = MockAuthoritySelectionDataSource::default()
			.with_candidates_per_epoch(vec![vec![], candidate_registrations.clone()])
			.with_permissioned_candidates(vec![None, Some(permissioned_candidates.clone())]);

		let registration = candidate_registrations.first().unwrap().clone();
		let expected_entry = CandidateRegistrationEntry::new(
			registration.registrations().first().unwrap().clone(),
			registration.mainchain_pub_key().clone(),
			Some(StakeDelegation(7)),
			None,
		);

		let expected_registrations = HashMap::from([(
			to_hex(&registration.mainchain_pub_key().0, false),
			vec![expected_entry],
		)]);

		let expected = AriadneParameters {
			d_parameter: types::DParameter {
				num_permissioned_candidates: 3,
				num_registered_candidates: 2,
			},
			permissioned_candidates: Some(
				permissioned_candidates
					.into_iter()
					.map(|data| PermissionedCandidateData::new(data, None))
					.collect(),
			),
			candidate_registrations: expected_registrations,
		};

		let client = Arc::new(TestApi {});
		let api =
			SessionValidatorManagementQuery::new(client, Arc::new(candidate_data_source_mock));

		let ariadne_parameters = api.get_ariadne_parameters(McEpochNumber(1)).await.unwrap();
		assert_eq!(ariadne_parameters, expected);
		for permissioned_candidate in ariadne_parameters.permissioned_candidates.unwrap() {
			assert!(permissioned_candidate.is_valid);
			assert!(permissioned_candidate.invalid_reasons.is_none());
		}

		let params_from_unsupported_epoch = api.get_ariadne_parameters(McEpochNumber(0)).await;

		assert!(params_from_unsupported_epoch.is_ok());
	}

	#[tokio::test]
	async fn validate_permissioned_candidate_data_should_return_error_if_invalid() {
		let invalid_permissioned_candidates = vec![
			sidechain_domain::PermissionedCandidateData {
				sidechain_public_key: SidechainPublicKey(vec![1; 32]),
				..valid_permissioned_candidate()
			},
			sidechain_domain::PermissionedCandidateData {
				aura_public_key: AuraPublicKey(vec![1; 37]),
				..valid_permissioned_candidate()
			},
			sidechain_domain::PermissionedCandidateData {
				grandpa_public_key: GrandpaPublicKey(vec![1; 33]),
				..valid_permissioned_candidate()
			},
		];

		// let candidate_registrations = create_candidates(vec![SEED], mock_sidechain_params());
		let candidate_data_source_mock = MockAuthoritySelectionDataSource::default()
			// .with_candidates_per_epoch(vec![vec![], candidate_registrations.clone()])
			.with_permissioned_candidates(vec![
				None,
				Some(invalid_permissioned_candidates.clone()),
			]);
		let client = Arc::new(TestApi {});
		let api =
			SessionValidatorManagementQuery::new(client, Arc::new(candidate_data_source_mock));
		let ariadne_parameters = api.get_ariadne_parameters(McEpochNumber(1)).await.unwrap();

		for permissioned_candidate in ariadne_parameters.permissioned_candidates.clone().unwrap() {
			assert!(!permissioned_candidate.is_valid);
		}
		match &ariadne_parameters.permissioned_candidates.unwrap()[..] {
			[first, second, third] => {
				assert_eq!(
					first.invalid_reasons,
					Some(PermissionedCandidateDataError::InvalidSidechainPubKey)
				);
				assert_eq!(
					second.invalid_reasons,
					Some(PermissionedCandidateDataError::InvalidAuraKey)
				);
				assert_eq!(
					third.invalid_reasons,
					Some(PermissionedCandidateDataError::InvalidGrandpaKey)
				);
			},
			_ => panic!("Expected 3 permissioned candidates"),
		}
	}
}
