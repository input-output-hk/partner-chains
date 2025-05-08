use crate::mock::*;
use crate::*;
use frame_support::testing_prelude::bounded_btree_map;
use frame_support::{assert_err, assert_ok};
use mock_pallet::HookCalls;

fn bstring<T: Get<u32>>(s: &str) -> BoundedString<T> {
	BoundedString::try_from(s).unwrap()
}

fn bvec<T: Get<u32>>(bytes: &[u8]) -> BoundedVec<u8, T> {
	BoundedVec::truncate_from(bytes.to_vec())
}

fn upsert(key: &str, value: &[u8]) -> (String, Option<ByteString>) {
	(key.into(), Some(value.to_vec().into()))
}

fn delete(key: &str) -> (String, Option<ByteString>) {
	(key.into(), None)
}

mod register_changes {
	use super::*;
	use pretty_assertions::assert_eq;

	#[test]
	fn inserts_entries() {
		new_test_ext().execute_with(|| {
			assert_ok!(GovernedMap::register_changes(
				RuntimeOrigin::none(),
				bounded_btree_map![
					bstring("key1") => Some(bvec(&[2; 23])),
					bstring("key2") => Some(bvec(&[1])),
					bstring("key3") => Some(bvec(&[8; 8])),
				]
			));

			assert_eq!(
				mappings_in_storage(),
				[
					(bstring("key1"), bvec(&[2; 23])),
					(bstring("key2"), bvec(&[1])),
					(bstring("key3"), bvec(&[8; 8])),
				]
			);

			assert_eq!(
				HookCalls::<Test>::take(),
				vec![
					MappingChange::Created { key: bstring("key1"), value: bvec(&[2; 23]) },
					MappingChange::Created { key: bstring("key2"), value: bvec(&[1]) },
					MappingChange::Created { key: bstring("key3"), value: bvec(&[8; 8]) },
				],
			);
		})
	}

	#[test]
	fn updates_entries() {
		new_test_ext().execute_with(|| {
			assert_ok!(GovernedMap::register_changes(
				RuntimeOrigin::none(),
				bounded_btree_map![
					bstring("key1") => Some(bvec(&[1])),
					bstring("key2") => Some(bvec(&[1, 2])),
					bstring("key3") => Some(bvec(&[1, 2, 3])),
				]
			));

			assert_eq!(
				HookCalls::<Test>::take(),
				vec![
					MappingChange::Created { key: bstring("key1"), value: bvec(&[1]) },
					MappingChange::Created { key: bstring("key2"), value: bvec(&[1, 2]) },
					MappingChange::Created { key: bstring("key3"), value: bvec(&[1, 2, 3]) },
				],
			);

			System::set_block_number(System::block_number() + 1);

			assert_ok!(GovernedMap::register_changes(
				RuntimeOrigin::none(),
				bounded_btree_map![
					bstring("key1") => Some(bvec(&[1, 1, 1])),
					bstring("key2") => Some(bvec(&[2])),
				]
			));

			assert_eq!(
				mappings_in_storage(),
				[
					(bstring("key1"), bvec(&[1, 1, 1])),
					(bstring("key2"), bvec(&[2])),
					(bstring("key3"), bvec(&[1, 2, 3]))
				]
			);

			assert_eq!(
				HookCalls::<Test>::take(),
				vec![
					MappingChange::Updated {
						key: bstring("key1"),
						old_value: bvec(&[1]),
						new_value: bvec(&[1, 1, 1]),
					},
					MappingChange::Updated {
						key: bstring("key2"),
						old_value: bvec(&[1, 2]),
						new_value: bvec(&[2]),
					},
				],
			);
		})
	}

	#[test]
	fn updates_entries_to_new_values_and_back_to_original() {
		new_test_ext().execute_with(|| {
			// Insert initial state
			{
				assert_ok!(GovernedMap::register_changes(
					RuntimeOrigin::none(),
					bounded_btree_map![
						bstring("key1") => Some(bvec(&[1, 1])),
						bstring("key2") => Some(bvec(&[2, 2])),
					]
				));

				assert_eq!(
					mappings_in_storage(),
					[(bstring("key1"), bvec(&[1, 1])), (bstring("key2"), bvec(&[2, 2])),]
				);

				assert_eq!(
					HookCalls::<Test>::take(),
					vec![
						MappingChange::Created { key: bstring("key1"), value: bvec(&[1, 1]) },
						MappingChange::Created { key: bstring("key2"), value: bvec(&[2, 2]) },
					],
				);
			}

			System::set_block_number(System::block_number() + 1);

			// Update to new values
			{
				assert_ok!(GovernedMap::register_changes(
					RuntimeOrigin::none(),
					bounded_btree_map![
						bstring("key1") => Some(bvec(&[3, 3])),
						bstring("key2") => Some(bvec(&[4, 4])),
					]
				));

				assert_eq!(
					HookCalls::<Test>::take(),
					vec![
						MappingChange::Updated {
							key: bstring("key1"),
							old_value: bvec(&[1, 1]),
							new_value: bvec(&[3, 3]),
						},
						MappingChange::Updated {
							key: bstring("key2"),
							old_value: bvec(&[2, 2]),
							new_value: bvec(&[4, 4]),
						},
					],
				);

				assert_eq!(
					mappings_in_storage(),
					[(bstring("key1"), bvec(&[3, 3])), (bstring("key2"), bvec(&[4, 4])),]
				);
			}

			System::set_block_number(System::block_number() + 1);

			// Update to the same values
			{
				assert_ok!(GovernedMap::register_changes(
					RuntimeOrigin::none(),
					bounded_btree_map![
						bstring("key1") => Some(bvec(&[3, 3])),
						bstring("key2") => Some(bvec(&[4, 4])),
					]
				));

				assert_eq!(
					HookCalls::<Test>::take(),
					vec![
						MappingChange::Updated {
							key: bstring("key1"),
							old_value: bvec(&[3, 3]),
							new_value: bvec(&[3, 3]),
						},
						MappingChange::Updated {
							key: bstring("key2"),
							old_value: bvec(&[4, 4]),
							new_value: bvec(&[4, 4]),
						},
					],
				);

				assert_eq!(
					mappings_in_storage(),
					[(bstring("key1"), bvec(&[3, 3])), (bstring("key2"), bvec(&[4, 4])),]
				);
			}

			System::set_block_number(System::block_number() + 1);

			// Update to the original values
			{
				assert_ok!(GovernedMap::register_changes(
					RuntimeOrigin::none(),
					bounded_btree_map![
						bstring("key1") => Some(bvec(&[1, 1])),
						bstring("key2") => Some(bvec(&[2, 2])),
					]
				));

				assert_eq!(
					HookCalls::<Test>::take(),
					vec![
						MappingChange::Updated {
							key: bstring("key1"),
							old_value: bvec(&[3, 3]),
							new_value: bvec(&[1, 1]),
						},
						MappingChange::Updated {
							key: bstring("key2"),
							old_value: bvec(&[4, 4]),
							new_value: bvec(&[2, 2]),
						},
					],
				);
			}
		})
	}

	#[test]
	fn deletes_entries() {
		new_test_ext().execute_with(|| {
			assert_ok!(GovernedMap::register_changes(
				RuntimeOrigin::none(),
				bounded_btree_map![
					bstring("key1") => Some(bvec(&[1])),
					bstring("key2") => Some(bvec(&[1, 2])),
					bstring("key3") => Some(bvec(&[1, 2, 3])),
				]
			));

			System::set_block_number(System::block_number() + 1);

			assert_ok!(GovernedMap::register_changes(
				RuntimeOrigin::none(),
				bounded_btree_map![
					bstring("key1") => None, bstring("key3") => None
				]
			));

			assert_eq!(mappings_in_storage(), [(bstring("key2"), bvec(&[1, 2]))]);
			assert_eq!(
				HookCalls::<Test>::take(),
				vec![
					MappingChange::Created { key: bstring("key1"), value: bvec(&[1]) },
					MappingChange::Created { key: bstring("key2"), value: bvec(&[1, 2]) },
					MappingChange::Created { key: bstring("key3"), value: bvec(&[1, 2, 3]) },
					MappingChange::Deleted { key: bstring("key1"), old_value: bvec(&[1]) },
					MappingChange::Deleted { key: bstring("key3"), old_value: bvec(&[1, 2, 3]) },
				],
			);
		})
	}

	#[test]
	fn reinitialize_the_governed_map_with_new_changes() {
		new_test_ext().execute_with(|| {
			assert_ok!(GovernedMap::register_changes(
				RuntimeOrigin::none(),
				bounded_btree_map![
					bstring("key1") => Some(bvec(&[1])),
					bstring("key2") => Some(bvec(&[1, 2])),
				]
			));

			assert_ok!(GovernedMap::set_main_chain_scripts(
				RuntimeOrigin::root(),
				MainChainScriptsV1::default()
			));

			System::set_block_number(System::block_number() + 1);

			assert_ok!(GovernedMap::register_changes(
				RuntimeOrigin::none(),
				bounded_btree_map![
					bstring("key1") => Some(bvec(&[2])),
					bstring("key2") => Some(bvec(&[3, 4])),
				]
			));

			assert_eq!(
				mappings_in_storage(),
				[(bstring("key1"), bvec(&[2])), (bstring("key2"), bvec(&[3, 4]))]
			);
			assert_eq!(
				HookCalls::<Test>::take(),
				vec![
					MappingChange::Created { key: bstring("key1"), value: bvec(&[1]) },
					MappingChange::Created { key: bstring("key2"), value: bvec(&[1, 2]) },
					MappingChange::Updated {
						key: bstring("key1"),
						old_value: bvec(&[1]),
						new_value: bvec(&[2]),
					},
					MappingChange::Updated {
						key: bstring("key2"),
						old_value: bvec(&[1, 2]),
						new_value: bvec(&[3, 4]),
					},
				],
			);
		});
	}

	#[test]
	fn if_main_chain_script_is_not_set_then_do_not_allow_for_changes_registration() {
		new_test_ext().execute_with(|| {
			assert_eq!(GovernedMap::get_main_chain_scripts(), None);

			// let mut inherent_data = InherentData::new();
			// assert_ok!(
			// 	inherent_data.put_data(INHERENT_IDENTIFIER, &GovernedMapInherentDataV1::default()),
			// );

			// assert_eq!(
			// 	GovernedMap::create_inherent(&inherent_data),
			// 	Some(Call::<Test>::register_changes { changes: bounded_btree_map![] })
			// );

			// assert_err!(
			// 	GovernedMap::register_changes(RuntimeOrigin::none(), bounded_btree_map![]),
			// 	Error::<Test>::MainChainScriptNotSet
			// );
		});
	}

	mod public_functions {
		use super::*;
		use pretty_assertions::assert_eq;

		#[test]
		fn get_key_value() {
			new_test_ext().execute_with(|| {
				assert_ok!(GovernedMap::register_changes(
					RuntimeOrigin::none(),
					bounded_btree_map![
						bstring("key1") => Some(bvec(&[1])),
						bstring("key2") => Some(bvec(&[1, 2])),
						bstring("key3") => Some(bvec(&[1, 2, 3])),
					]
				));

				assert_eq!(GovernedMap::get_key_value(&bstring("key1")), Some(bvec(&[1])));
				assert_eq!(GovernedMap::get_key_value(&bstring("key2")), Some(bvec(&[1, 2])));
				assert_eq!(GovernedMap::get_key_value(&bstring("key3")), Some(bvec(&[1, 2, 3])));

				assert_eq!(GovernedMap::get_key_value(&bstring("key0")), None);
			});
		}

		#[test]
		fn get_all_key_value_pairs() {
			new_test_ext().execute_with(|| {
				assert_eq!(GovernedMap::get_all_key_value_pairs().next(), None);

				assert_ok!(GovernedMap::register_changes(
					RuntimeOrigin::none(),
					bounded_btree_map![
						bstring("key1") => Some(bvec(&[1, 2])),
					]
				));

				assert_eq!(
					GovernedMap::get_all_key_value_pairs().collect::<Vec<_>>(),
					vec![(bstring("key1"), bvec(&[1, 2]))]
				);
			});
		}

		#[test]
		fn get_all_key_value_pairs_unbounded() {
			new_test_ext().execute_with(|| {
				assert_eq!(GovernedMap::get_all_key_value_pairs().next(), None);

				assert_ok!(GovernedMap::register_changes(
					RuntimeOrigin::none(),
					bounded_btree_map![
						bstring("key1") => Some(bvec(&[1, 2])),
					]
				));

				assert_eq!(
					GovernedMap::get_all_key_value_pairs_unbounded().collect::<Vec<_>>(),
					vec![("key1".to_string(), ByteString::from(vec![1, 2]))]
				);
			});
		}

		#[test]
		fn is_initialized() {
			new_test_ext().execute_with(|| {
				assert!(!GovernedMap::is_initialized());

				assert_ok!(GovernedMap::set_main_chain_scripts(
					RuntimeOrigin::root(),
					MainChainScriptsV1::default()
				));

				System::set_block_number(System::block_number() + 1);

				assert_ok!(GovernedMap::register_changes(
					RuntimeOrigin::none(),
					bounded_btree_map![
						bstring("key1") => Some(bvec(&[1, 2])),
					]
				));

				assert!(GovernedMap::is_initialized());
			});
		}

		#[test]
		fn get_version() {
			new_test_ext().execute_with(|| {
				assert_eq!(GovernedMap::get_version(), PALLET_VERSION);
			});
		}

		#[test]
		fn get_main_chain_scripts() {
			new_test_ext().execute_with(|| {
				assert_eq!(GovernedMap::get_main_chain_scripts(), None);

				assert_ok!(GovernedMap::set_main_chain_scripts(
					RuntimeOrigin::root(),
					MainChainScriptsV1::default()
				));

				assert_eq!(
					GovernedMap::get_main_chain_scripts(),
					Some(MainChainScriptsV1::default())
				);
			});
		}
	}

	#[test]
	fn cannot_be_called_more_than_once_per_block() {
		new_test_ext().execute_with(|| {
			let changes = bounded_btree_map![
				bstring("key1") => Some(bvec(&[1])),
			];
			assert_ok!(GovernedMap::register_changes(RuntimeOrigin::none(), changes.clone()));
			assert_err!(
				GovernedMap::register_changes(RuntimeOrigin::none(), changes),
				Error::<Test>::InherentCalledTwice,
			);

			System::set_block_number(System::block_number() + 1);

			assert_ok!(GovernedMap::register_changes(
				RuntimeOrigin::none(),
				bounded_btree_map![
					bstring("key2") => Some(bvec(&[2])),
				]
			));
		})
	}

	mod provide_inherent {
		use super::*;
		use pretty_assertions::assert_eq;

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
		fn does_not_require_an_inherent_when_data_is_empty_and_pallet_is_initialized() {
			new_test_ext().execute_with(|| {
				Initialized::<Test>::put(true);
				let mut inherent_data = InherentData::new();
				inherent_data
					.put_data(INHERENT_IDENTIFIER, &GovernedMapInherentDataV1::new())
					.unwrap();
				assert!(
					GovernedMap::is_inherent_required(&inherent_data)
						.expect("Should not fail")
						.is_none()
				)
			})
		}

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
					changes: bounded_btree_map![
						bstring("key1") => Some(bvec(&[1])),
						bstring("key2") => Some(bvec(&[2])),
						bstring("key3") => None
					],
				}
			);
		}

		#[test]
		fn if_pallet_is_initialized_and_data_is_empty_then_we_skip_inherent() {
			new_test_ext().execute_with(|| {
				// Initialize pallet with some data
				assert_ok!(GovernedMap::register_changes(
					RuntimeOrigin::none(),
					bounded_btree_map![
						bstring("key1") => Some(bvec(&[1])),
					]
				));

				assert!(GovernedMap::is_initialized());

				let mut inherent_data = InherentData::new();
				assert_ok!(
					inherent_data
						.put_data(INHERENT_IDENTIFIER, &GovernedMapInherentDataV1::default()),
				);

				assert_eq!(GovernedMap::create_inherent(&inherent_data), None);
			});
		}

		#[test]
		fn if_new_script_is_set_and_there_is_no_diff_then_we_allow_for_adding_of_empty_extrinsic_to_annotate_this()
		 {
			new_test_ext().execute_with(|| {
				assert!(!GovernedMap::is_initialized());

				// Initialize pallet with some data
				assert_ok!(GovernedMap::register_changes(
					RuntimeOrigin::none(),
					bounded_btree_map![
						bstring("key1") => Some(bvec(&[1])),
					]
				));

				assert!(GovernedMap::is_initialized());

				assert_ok!(GovernedMap::set_main_chain_scripts(
					RuntimeOrigin::root(),
					MainChainScriptsV1::default()
				));

				assert!(!GovernedMap::is_initialized());

				System::set_block_number(System::block_number() + 1);

				let mut inherent_data = InherentData::new();
				assert_ok!(
					inherent_data
						.put_data(INHERENT_IDENTIFIER, &GovernedMapInherentDataV1::default()),
				);

				assert_eq!(
					GovernedMap::create_inherent(&inherent_data),
					Some(Call::<Test>::register_changes { changes: bounded_btree_map![] })
				);
			});
		}

		#[test]
		fn if_we_start_from_scratch_and_there_are_no_changes_then_we_allow_to_once_register_them() {
			new_test_ext().execute_with(|| {
				// Pallet is not initialized from the beginning
				assert!(!GovernedMap::is_initialized());

				let mut inherent_data = InherentData::new();
				assert_ok!(
					inherent_data
						.put_data(INHERENT_IDENTIFIER, &GovernedMapInherentDataV1::default()),
				);

				assert_eq!(
					GovernedMap::create_inherent(&inherent_data),
					Some(Call::<Test>::register_changes { changes: bounded_btree_map![] })
				);
			});
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
		fn rejects_inherent_when_data_missing() {
			let inherent_data = InherentData::new();

			let inherent = Call::<Test>::register_changes {
				changes: bounded_btree_map![bstring("key2") => Some(bvec(&[1, 2, 3]))],
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
				changes: bounded_btree_map![bstring("key2") => Some(bvec(&[1, 2, 3]))],
			};

			let err = GovernedMap::check_inherent(&inherent, &inherent_data)
				.expect_err("Should return an error");

			assert_eq!(err, InherentError::IncorrectInherent);
		}

		#[test]
		#[should_panic(expected = "TooManyChanges")]
		fn fails_when_change_number_exceeds_limit() {
			let data_changes: GovernedMapInherentDataV1 =
				(0..=TEST_MAX_CHANGES).map(|i| upsert(&format!("key{i}"), &[1])).collect();

			let mut inherent_data = InherentData::new();
			inherent_data.put_data(INHERENT_IDENTIFIER, &data_changes).unwrap();

			GovernedMap::create_inherent(&inherent_data);
		}
	}
}
