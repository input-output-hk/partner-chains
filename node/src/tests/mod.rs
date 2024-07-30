mod inherent_data_tests;
mod mock;
mod runtime_api_mock;

use self::chain_init::EpochTimer;

use super::*;
use crate::{chain_init::Sleeper, chain_spec::AuthorityKeys};
use async_trait::async_trait;
use derive_new::new;
use main_chain_follower_api::mock_services::MockCandidateDataSource;
use main_chain_follower_api::mock_services::TestDataSources;
use mock::*;
use sidechain_domain::McEpochNumber;
use sidechain_runtime::opaque::SessionKeys;
use sidechain_runtime::CrossChainPublic;
use sp_core::{ecdsa, ed25519, sr25519};
use sp_session_validator_management::MainChainScripts;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::time::Duration;

#[derive(Clone, new)]
struct CountingSleeper {
	#[new(default)]
	count: Arc<Mutex<u32>>,
}

#[async_trait]
impl Sleeper for CountingSleeper {
	async fn sleep(&self, _duration: Duration) {
		let mut count = self.count.lock().unwrap();
		*count += 1;
	}
}

#[derive(new)]
struct EpochTimerMock {
	#[new(default)]
	current_mock_epoch: Arc<Mutex<u32>>,
}

#[async_trait]
impl EpochTimer for EpochTimerMock {
	fn get_current_epoch(&self) -> Result<McEpochNumber, String> {
		let mut current_mock_epoch = self.current_mock_epoch.lock().unwrap();
		let res = McEpochNumber(*current_mock_epoch);
		*current_mock_epoch += 1;
		Ok(res)
	}
}

fn session_keys_from((aura, grandpa): ([u8; 32], [u8; 32])) -> SessionKeys {
	let aura = sr25519::Public::from_raw(aura).into();
	let grandpa = ed25519::Public::from_raw(grandpa).into();
	SessionKeys { aura, grandpa }
}

#[tokio::test]
async fn chain_init_success_on_the_first_attempt() {
	let mock_committee = vec![
		(
			[1u8; 32],              // account seed
			[0u8; 33],              // cross chain public
			([2u8; 32], [3u8; 32]), // aura and grandpa keys
		),
		(
			[2u8; 32],              // account seed
			[2u8; 33],              // cross chain public
			([5u8; 32], [6u8; 32]), // aura and grandpa keys
		),
	];

	let client = Arc::new(runtime_api_mock::TestApi::default());

	let candidates_to_return_on_the_1st_call = create_candidates(mock_committee.clone());
	let all_candidates = vec![candidates_to_return_on_the_1st_call];

	let candidate_data_source_mock = MockCandidateDataSource::default()
		.with_candidates_per_epoch(all_candidates)
		.with_permissioned_candidates(vec![Some(vec![])]);
	let mock_main_chain_follower_data_sources = TestDataSources::new()
		.with_candidate_data_source(candidate_data_source_mock)
		.into();

	let sleeper = CountingSleeper::new();

	let result = chain_init::get_initial_authorities_with_waits(
		client,
		&mock_main_chain_follower_data_sources,
		McEpochNumber(0),
		sleeper.clone(),
		EpochTimerMock::new(),
	)
	.await
	.unwrap();
	let expected_result: Vec<AuthorityKeys> = mock_committee
		.into_iter()
		.map(|(_, public, session_keys)| AuthorityKeys {
			cross_chain: CrossChainPublic::from(ecdsa::Public::from(public)),
			session: session_keys_from(session_keys),
		})
		.collect();
	assert_eq!(result, expected_result);
	assert_eq!(*sleeper.count.lock().unwrap(), 0); // 0 retries, 1st call is successful
}

#[tokio::test]
// Test that, if our `minimum_mc_epoch` is set earlier than the earliest candidate list, we can still
// find the committee
async fn chain_init_success_after_10_retries() {
	let mock_committee = vec![
		(
			[1u8; 32],              // account seed
			[0u8; 33],              // cross chain public
			([2u8; 32], [3u8; 32]), // aura and grandpa keys
		),
		(
			[2u8; 32],              // account seed
			[1u8; 33],              // cross chain public
			([5u8; 32], [6u8; 32]), // aura and grandpa keys
		),
	];

	let client = Arc::new(runtime_api_mock::TestApi::default());

	let candidates_to_return_on_the_10th_call = create_candidates(mock_committee.clone());

	let all_candidates = vec![vec![]; 9]
		.into_iter()
		.chain(vec![candidates_to_return_on_the_10th_call])
		.collect::<Vec<_>>();

	let candidate_data_source_mock = MockCandidateDataSource::default()
		.with_candidates_per_epoch(all_candidates)
		.with_permissioned_candidates(vec![Some(vec![]); 10]);
	let mock_main_chain_follower_data_sources = TestDataSources::new()
		.with_candidate_data_source(candidate_data_source_mock)
		.into();

	let sleeper = CountingSleeper { count: Arc::new(Mutex::new(0)) };

	let result = chain_init::get_initial_authorities_with_waits(
		client,
		&mock_main_chain_follower_data_sources,
		McEpochNumber(0),
		sleeper.clone(),
		EpochTimerMock::new(),
	)
	.await
	.unwrap();
	let expected_result: Vec<AuthorityKeys> = mock_committee
		.into_iter()
		.map(|(_, public, session_keys)| AuthorityKeys {
			cross_chain: CrossChainPublic::from(ecdsa::Public::from(public)),
			session: session_keys_from(session_keys),
		})
		.collect();

	assert_eq!(result, expected_result);
	assert_eq!(*sleeper.count.lock().unwrap(), 9); // 9 retries, 10th call is successful
}

#[tokio::test]
// Test that, if our `minimum_mc_epoch` is set to the earliest epoch at which there exists a candidates
// list, that list is selected
async fn chain_init_first_available() {
	let mock_committee_1 = vec![
		(
			[1u8; 32],              // account seed
			[0u8; 33],              // cross chain public
			([2u8; 32], [3u8; 32]), // aura and grandpa keys
		),
		(
			[2u8; 32],              // account seed
			[1u8; 33],              // cross chain public
			([5u8; 32], [6u8; 32]), // aura and grandpa keys
		),
	];

	let mock_committee_2 = vec![
		(
			[1u8; 32],              // account seed
			[0u8; 33],              // cross chain public
			([2u8; 32], [3u8; 32]), // aura and grandpa keys
		),
		(
			[2u8; 32],              // account seed
			[1u8; 33],              // cross chain public
			([5u8; 32], [6u8; 32]), // aura and grandpa keys
		),
		(
			[3u8; 32],              // account seed
			[2u8; 33],              // cross chain public
			([8u8; 32], [9u8; 32]), // aura and grandpa keys
		),
	];

	let client = Arc::new(runtime_api_mock::TestApi::default());

	let candidates_to_return_on_the_9th_call = create_candidates(mock_committee_1.clone());
	let candidates_to_return_on_the_10th_call = create_candidates(mock_committee_2.clone());

	let all_candidates = vec![vec![]; 8]
		.into_iter()
		.chain(vec![candidates_to_return_on_the_9th_call])
		.chain(vec![candidates_to_return_on_the_10th_call])
		.collect::<Vec<_>>();

	let candidate_data_source_mock = MockCandidateDataSource::default()
		.with_candidates_per_epoch(all_candidates)
		.with_permissioned_candidates(vec![Some(vec![]); 10]);
	let mock_main_chain_follower_data_sources = TestDataSources::new()
		.with_candidate_data_source(candidate_data_source_mock)
		.into();

	let sleeper = CountingSleeper { count: Arc::new(Mutex::new(0)) };

	let result = chain_init::get_initial_authorities_with_waits(
		client,
		&mock_main_chain_follower_data_sources,
		// Sleep 8 times due to minimum epoch number.
		McEpochNumber(8),
		sleeper.clone(),
		EpochTimerMock::new(),
	)
	.await
	.unwrap();
	let expected_result: Vec<AuthorityKeys> = mock_committee_1
		.into_iter()
		.map(|(_, public, session_keys)| AuthorityKeys {
			cross_chain: CrossChainPublic::from(ecdsa::Public::from(public)),
			session: session_keys_from(session_keys),
		})
		.collect();
	println!("expected: {:?}", expected_result);

	assert_eq!(result, expected_result);
	assert_eq!(*sleeper.count.lock().unwrap(), 8); // 8 sleeps, 9th call is successful
}

#[tokio::test]
// Test that, if our `minimum_mc_epoch` is ahead of the earliest candidates list, we'll correctly
// find the earliest list without waiting for the minimum
async fn chain_init_minimum_epoch_ahead_of_current_no_backtrack() {
	let mock_committee_1 = vec![
		(
			[1u8; 32],              // account seed
			[0u8; 33],              // cross chain public
			([2u8; 32], [3u8; 32]), // aura and grandpa keys
		),
		(
			[2u8; 32],              // account seed
			[1u8; 33],              // cross chain public
			([5u8; 32], [6u8; 32]), // aura and grandpa keys
		),
	];

	let mock_committee_2 = vec![
		(
			[1u8; 32],              // account seed
			[0u8; 33],              // cross chain public
			([2u8; 32], [3u8; 32]), // aura and grandpa keys
		),
		(
			[2u8; 32],              // account seed
			[1u8; 33],              // cross chain public
			([5u8; 32], [6u8; 32]), // aura and grandpa keys
		),
		(
			[3u8; 32],              // account seed
			[2u8; 33],              // cross chain public
			([8u8; 32], [9u8; 32]), // aura and grandpa keys
		),
	];

	let client = Arc::new(runtime_api_mock::TestApi::default());

	let candidates_to_return_on_the_9th_call = create_candidates(mock_committee_1.clone());
	let candidates_to_return_on_the_10th_call = create_candidates(mock_committee_2.clone());

	let all_candidates = vec![vec![]; 8]
		.into_iter()
		.chain(vec![candidates_to_return_on_the_9th_call])
		.chain(vec![candidates_to_return_on_the_10th_call])
		.collect::<Vec<_>>();

	let candidate_data_source_mock = MockCandidateDataSource::default()
		.with_candidates_per_epoch(all_candidates)
		.with_permissioned_candidates(vec![Some(vec![]); 10]);
	let mock_main_chain_follower_data_sources = TestDataSources::new()
		.with_candidate_data_source(candidate_data_source_mock)
		.into();

	let sleeper = CountingSleeper { count: Arc::new(Mutex::new(0)) };

	let result = chain_init::get_initial_authorities_with_waits(
		client,
		&mock_main_chain_follower_data_sources,
		// Ignored because we're not starting in the future
		McEpochNumber(9),
		sleeper.clone(),
		EpochTimerMock::new(),
	)
	.await
	.unwrap();

	let expected_result: Vec<AuthorityKeys> = mock_committee_1
		.into_iter()
		.map(|(_, public, session_keys)| AuthorityKeys {
			cross_chain: CrossChainPublic::from(ecdsa::Public::from(public)),
			session: session_keys_from(session_keys),
		})
		.collect();

	assert_eq!(result, expected_result);
	assert_eq!(*sleeper.count.lock().unwrap(), 8); // 8 sleeps, 9th call is successful
}

#[tokio::test]
// Test that, if we're a late joiner, we can find the earliest committee by backtracking
async fn chain_init_minimum_epoch_ahead_of_current_then_backtrack() {
	let mock_committee_0 = vec![(
		[1u8; 32],              // account seed
		[0u8; 33],              // cross chain public
		([2u8; 32], [3u8; 32]), // aura and grandpa keys
	)];

	let mock_committee_1 = vec![
		(
			[1u8; 32],              // account seed
			[0u8; 33],              // cross chain public
			([2u8; 32], [3u8; 32]), // aura and grandpa keys
		),
		(
			[2u8; 32],              // account seed
			[1u8; 33],              // cross chain public
			([5u8; 32], [6u8; 32]), // aura and grandpa keys
		),
	];

	let mock_committee_2 = vec![
		(
			[1u8; 32],              // account seed
			[0u8; 33],              // cross chain public
			([2u8; 32], [3u8; 32]), // aura and grandpa keys
		),
		(
			[2u8; 32],              // account seed
			[1u8; 33],              // cross chain public
			([5u8; 32], [6u8; 32]), // aura and grandpa keys
		),
		(
			[3u8; 32],              // account seed
			[2u8; 33],              // cross chain public
			([8u8; 32], [9u8; 32]), // aura and grandpa keys
		),
	];

	let client = Arc::new(runtime_api_mock::TestApi::default());

	let candidates_1 = create_candidates(mock_committee_0.clone());
	let candidates_2 = create_candidates(mock_committee_1.clone());
	let candidates_3 = create_candidates(mock_committee_2.clone());

	let all_candidates = vec![]
		.into_iter()
		.chain(vec![candidates_1])
		.chain(vec![candidates_2])
		.chain(vec![candidates_3])
		.collect::<Vec<_>>();

	let candidate_data_source_mock = MockCandidateDataSource::default()
		.with_candidates_per_epoch(all_candidates)
		.with_permissioned_candidates(vec![Some(vec![]); 10]);
	let mock_main_chain_follower_data_sources = TestDataSources::new()
		.with_candidate_data_source(candidate_data_source_mock)
		.into();

	let sleeper = CountingSleeper { count: Arc::new(Mutex::new(0)) };
	let epoch_mock = EpochTimerMock { current_mock_epoch: Arc::new(Mutex::new(3)) };

	let result = chain_init::get_initial_authorities_with_waits(
		client,
		&mock_main_chain_follower_data_sources,
		McEpochNumber(2),
		sleeper.clone(),
		epoch_mock,
	)
	.await
	.unwrap();

	let expected_result: Vec<AuthorityKeys> = mock_committee_0
		.into_iter()
		.map(|(_, public, session_keys)| AuthorityKeys {
			cross_chain: CrossChainPublic::from(ecdsa::Public::from(public)),
			session: session_keys_from(session_keys),
		})
		.collect();

	assert_eq!(result, expected_result);
	assert_eq!(*sleeper.count.lock().unwrap(), 0); // No retries expected, since the list definitely exists
}

#[test]
fn should_insert_validators_into_storage() {
	let mock_committee = vec![
		(
			[0u8; 33],              // cross chain public
			([2u8; 32], [3u8; 32]), // aura and grandpa keys
		),
		(
			[1u8; 33],              // cross chain public
			([5u8; 32], [6u8; 32]), // aura and grandpa keys
		),
	];

	let committee: Vec<AuthorityKeys> = mock_committee
		.into_iter()
		.map(|(public, session_keys)| AuthorityKeys {
			cross_chain: CrossChainPublic::from(ecdsa::Public::from(public)),
			session: session_keys_from(session_keys),
		})
		.collect();

	let mut storage = sp_core::storage::Storage::default();
	let main_chain_scripts = MainChainScripts::default();
	chain_init::insert_into_storage(committee.clone(), main_chain_scripts, &mut storage).unwrap();

	// 3 fields in pallet-session-committee-management and 3 in partner-chains-session-pallet.
	// Encoding keys and values according to substrate storage layout is quite involved,
	// so we just check the number of elements in the storage.

	// TODO: temporarily disabled
	// assert_eq!(storage.top.len(), 6);

	// For future reference, here's how to encode "Committee" key:
	//
	// let session_committee_management_key =
	// 	&sp_core::hashing::twox_128(b"SessionCommitteeManagement");
	// let committee_key = &sp_core::hashing::twox_128(b"Committee");
	// let epoch_key_encoded = ScEpochNumber::zero().using_encoded(|x| x.to_vec());
	// let epoch_key_hashed = sp_core::hashing::twox_64(&epoch_key_encoded);
	//
	// session_committee_management_key
	//   .iter()
	//   .chain(committee_key.iter())
	//   .chain(epoch_key_hashed.iter())
	//   .chain(epoch_key_encoded.iter())
	//   .cloned()
	//   .collect::<Vec<_>>()
}
