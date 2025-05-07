#![allow(missing_docs)]

mod idp_v1 {
	use crate::{GovernedMapInherentDataProvider, mock::*, *};
	use pretty_assertions::assert_eq;
	use sidechain_domain::McBlockHash;
	use sp_inherents::{InherentData, InherentDataProvider};
	use sp_runtime::{bounded_vec, traits::Block as BlockT};

	#[tokio::test]
	async fn calculates_changes_and_returns_active_when_pallet_needs_initializing() {
		let api = TestApiV1::uninitialized().with_current_state(
			[
				("deleted_key".into(), vec![1, 2, 3, 4].into()),
				("unchanged_key".into(), vec![1, 2, 3].into()),
			]
			.into(),
		);
		let data_source = MockGovernedMapDataSource {
			data: [
				("unchanged_key".into(), vec![1, 2, 3].into()),
				("inserted_key_1".into(), vec![1, 2, 3, 4].into()),
				("inserted_key_2".into(), vec![0, 0, 0, 0].into()),
			]
			.into(),
			..Default::default()
		};

		let idp = GovernedMapInherentDataProvider::new(
			&api,
			<Block as BlockT>::Hash::default(),
			McBlockHash::default(),
			Some(McBlockHash::default()),
			&data_source,
		)
		.await
		.expect("Should succeed");

		let GovernedMapInherentDataProvider::ActiveV1 { data } = idp else {
			panic!("IDP should be active")
		};

		let expected_changes = [
			("deleted_key".into(), None),
			("inserted_key_1".into(), Some(vec![1, 2, 3, 4].into())),
			("inserted_key_2".into(), Some(vec![0, 0, 0, 0].into())),
		]
		.into();

		assert_eq!(data, expected_changes);
	}

	#[tokio::test]
	async fn calculates_changes_and_returns_active_if_non_empty() {
		let api = TestApiV1::initialized();
		let data_source = MockGovernedMapDataSource {
			changes: vec![
				("updated_key".into(), Some(vec![63].into())),
				("inserted_key".into(), Some(vec![1, 2, 3].into())),
				("deleted_key".into(), None),
			]
			.into(),
			..Default::default()
		};

		let idp = GovernedMapInherentDataProvider::new(
			&api,
			<Block as BlockT>::Hash::default(),
			McBlockHash::default(),
			Some(McBlockHash::default()),
			&data_source,
		)
		.await
		.expect("Should succeed");

		let GovernedMapInherentDataProvider::ActiveV1 { data } = idp else {
			panic!("IDP should be active")
		};

		let expected_changes = [
			("updated_key".into(), Some(bounded_vec![63])),
			("inserted_key".into(), Some(bounded_vec![1, 2, 3])),
			("deleted_key".into(), None),
		]
		.into();

		assert_eq!(data, expected_changes);
	}

	#[tokio::test]
	async fn is_empty_when_there_are_no_changes() {
		let api = TestApiV1::initialized();
		let data_source = MockGovernedMapDataSource::default();

		let idp = GovernedMapInherentDataProvider::new(
			&api,
			<Block as BlockT>::Hash::default(),
			McBlockHash::default(),
			Some(McBlockHash::default()),
			&data_source,
		)
		.await
		.expect("Should succeed");

		assert_eq!(idp, GovernedMapInherentDataProvider::ActiveV1 { data: [].into() });
	}

	#[tokio::test]
	async fn provides_inherent_data_when_non_empty() {
		let changes: BTreeMap<_, _> = [
			("deleted_key".into(), None),
			("inserted_key".into(), Some(bounded_vec![1, 2, 3])),
			("updated_key".into(), Some(bounded_vec![63])),
		]
		.into();

		let idp = GovernedMapInherentDataProvider::ActiveV1 { data: changes.clone() };

		let mut inherent_data = InherentData::new();
		idp.provide_inherent_data(&mut inherent_data).await.unwrap();

		assert_eq!(
			inherent_data
				.get_data::<GovernedMapInherentDataV1>(&INHERENT_IDENTIFIER)
				.expect("Should succeed")
				.expect("Data should be present"),
			changes
		);
	}

	#[tokio::test]
	async fn does_not_provide_inherent_data_when_inert() {
		let idp = GovernedMapInherentDataProvider::Inert;

		let mut inherent_data = InherentData::new();
		idp.provide_inherent_data(&mut inherent_data).await.unwrap();

		assert!(
			inherent_data
				.get_data::<GovernedMapInherentDataV1>(&INHERENT_IDENTIFIER)
				.expect("Should succeed")
				.is_none()
		);
	}
}
