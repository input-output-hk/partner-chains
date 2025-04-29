use crate::mock::*;
use crate::*;
use frame_support::traits::UnfilteredDispatchable;
use sp_core::bounded_vec;

fn bstring<T: Get<u32>>(s: &str) -> BoundedString<T> {
	BoundedString::try_from(s).unwrap()
}

fn bvec<T: Get<u32>>(bytes: &[u8]) -> BoundedVec<u8, T> {
	BoundedVec::truncate_from(bytes.to_vec())
}

fn upsert(key: &str, value: &[u8]) -> GovernedMapChangeV1 {
	GovernedMapChangeV1 { key: key.into(), new_value: Some(value.to_vec().into()) }
}

fn delete(key: &str) -> GovernedMapChangeV1 {
	GovernedMapChangeV1 { key: key.into(), new_value: None }
}

mod inherent {
	use super::*;
	use pretty_assertions::assert_eq;

	#[test]
	fn inserts_entries() {
		new_test_ext().execute_with(|| {
			Mapping::<Test>::set(bstring("key0"), Some(bvec(&[0])));

			Call::<Test>::register_changes {
				changes: bounded_vec![
					(bstring("key1"), Some(bvec(&[2; 23]))),
					(bstring("key2"), Some(bvec(&[1]))),
					(bstring("key3"), Some(bvec(&[8; 8]))),
				],
			}
			.dispatch_bypass_filter(RuntimeOrigin::none())
			.expect("Should succeed");

			let mut mappings_in_storage = Mapping::<Test>::iter().collect::<Vec<_>>();
			mappings_in_storage.sort_by_key(|m| m.0.to_string());

			assert_eq!(
				mappings_in_storage,
				vec![
					(bstring("key0"), bvec(&[0])),
					(bstring("key1"), bvec(&[2; 23])),
					(bstring("key2"), bvec(&[1])),
					(bstring("key3"), bvec(&[8; 8])),
				]
			)
		})
	}

	#[test]
	fn updates_entries() {
		new_test_ext().execute_with(|| {
			Mapping::<Test>::set(bstring("key1"), Some(bvec(&[1])));
			Mapping::<Test>::set(bstring("key2"), Some(bvec(&[1, 2])));
			Mapping::<Test>::set(bstring("key3"), Some(bvec(&[1, 2, 3])));

			Call::<Test>::register_changes {
				changes: bounded_vec![
					(bstring("key1"), Some(bvec(&[1, 1, 1]))),
					(bstring("key2"), Some(bvec(&[2]))),
				],
			}
			.dispatch_bypass_filter(RuntimeOrigin::none())
			.expect("Should succeed");

			let mut mappings_in_storage = Mapping::<Test>::iter().collect::<Vec<_>>();
			mappings_in_storage.sort_by_key(|m| m.0.to_string());

			assert_eq!(
				mappings_in_storage,
				vec![
					(bstring("key1"), bvec(&[1, 1, 1])),
					(bstring("key2"), bvec(&[2])),
					(bstring("key3"), bvec(&[1, 2, 3])),
				]
			)
		})
	}

	#[test]
	fn deletes_entries() {
		new_test_ext().execute_with(|| {
			Mapping::<Test>::set(bstring("key1"), Some(bvec(&[1])));
			Mapping::<Test>::set(bstring("key2"), Some(bvec(&[1, 2])));
			Mapping::<Test>::set(bstring("key3"), Some(bvec(&[1, 2, 3])));

			Call::<Test>::register_changes {
				changes: bounded_vec![(bstring("key1"), None), (bstring("key3"), None)],
			}
			.dispatch_bypass_filter(RuntimeOrigin::none())
			.expect("Should succeed");

			let mappings_in_storage = Mapping::<Test>::iter().collect::<Vec<_>>();

			assert_eq!(mappings_in_storage, vec![(bstring("key2"), bvec(&[1, 2])),])
		})
	}

	#[test]
	fn calls_the_on_change_hook() {
		new_test_ext().execute_with(|| {
			Mapping::<Test>::set(bstring("key1"), Some(bvec(&[1])));
			Mapping::<Test>::set(bstring("key2"), Some(bvec(&[1, 2])));

			Call::<Test>::register_changes {
				changes: bounded_vec![
					(bstring("key1"), Some(bvec(&[1, 1, 1]))),
					(bstring("key2"), None),
					(bstring("key3"), Some(bvec(&[2]))),
				],
			}
			.dispatch_bypass_filter(RuntimeOrigin::none())
			.expect("Should succeed");

			assert_eq!(
				mock_pallet::HookCalls::<Test>::get(),
				vec![
					(bstring("key1"), Some(bvec(&[1, 1, 1])), Some(bvec(&[1]))),
					(bstring("key2"), None, Some(bvec(&[1, 2]))),
					(bstring("key3"), Some(bvec(&[2])), None),
				]
			)
		})
	}

	mod provide_inherent {
		use super::*;
		use pretty_assertions::assert_eq;

		#[test]
		fn creates_inherent() {
			let mut inherent_data = InherentData::new();

			let changes = vec![upsert("key1", &[1]), upsert("key2", &[2]), delete("key3")];

			inherent_data.put_data(INHERENT_IDENTIFIER, &changes).unwrap();

			let inherent =
				GovernedMap::create_inherent(&inherent_data).expect("Should produce inherent");

			assert_eq!(
				inherent,
				Call::<Test>::register_changes {
					changes: bounded_vec![
						(bstring("key1"), Some(bvec(&[1]))),
						(bstring("key2"), Some(bvec(&[2]))),
						(bstring("key3"), None)
					],
				}
			);
		}

		#[test]
		fn requires_an_inherent_when_data_present() {
			let mut inherent_data = InherentData::new();

			let changes = vec![upsert("key1", &[1]), upsert("key2", &[2]), delete("key3")];

			inherent_data.put_data(INHERENT_IDENTIFIER, &changes).unwrap();

			GovernedMap::is_inherent_required(&inherent_data)
				.expect("Should not fail")
				.expect("Should return a wrapped error");
		}

		#[test]
		fn does_not_require_an_inherent_when_data_not_present() {
			let inherent_data = InherentData::new();
			assert!(
				GovernedMap::is_inherent_required(&inherent_data)
					.expect("Should not fail")
					.is_none()
			)
		}

		#[test]
		fn rejects_inherent_when_data_missing() {
			let inherent_data = InherentData::new();

			let inherent = Call::<Test>::register_changes {
				changes: bounded_vec![(bstring("key2"), Some(bvec(&[1, 2, 3])))],
			};

			let err = GovernedMap::check_inherent(&inherent, &inherent_data)
				.expect_err("Should return an error");

			assert_eq!(err, InherentError::InherentNotExpected);
		}

		#[test]
		fn rejects_inherent_when_data_differs() {
			let data_changes = vec![upsert("key2", &[4, 2])];

			let mut inherent_data = InherentData::new();
			inherent_data.put_data(INHERENT_IDENTIFIER, &data_changes).unwrap();

			let inherent = Call::<Test>::register_changes {
				changes: bounded_vec![(bstring("key2"), Some(bvec(&[1, 2, 3])))],
			};

			let err = GovernedMap::check_inherent(&inherent, &inherent_data)
				.expect_err("Should return an error");

			assert_eq!(err, InherentError::IncorrectInherent);
		}

		#[test]
		#[should_panic(expected = "TooManyChanges")]
		fn fails_when_change_number_exceeds_limit() {
			let data_changes: Vec<_> =
				(0..=TEST_MAX_CHANGES).map(|i| upsert(&format!("key{i}"), &[1])).collect();

			let mut inherent_data = InherentData::new();
			inherent_data.put_data(INHERENT_IDENTIFIER, &data_changes).unwrap();

			GovernedMap::create_inherent(&inherent_data);
		}
	}
}
