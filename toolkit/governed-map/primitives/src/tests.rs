#![allow(missing_docs)]

impl crate::GovernedMapChangeV1 {
	pub fn upsert(key: &str, new_value: &[u8]) -> Self {
		Self { key: key.into(), new_value: Some(new_value.into()) }
	}
	pub fn delete(key: &str) -> Self {
		Self { key: key.into(), new_value: None }
	}
}

mod idp_v1 {
	use crate::{mock::*, GovernedMapInherentDataProvider, *};
	use sidechain_domain::McBlockHash;
	use sp_inherents::{InherentData, InherentDataProvider};
	use sp_runtime::traits::Block as BlockT;

	#[tokio::test]
	async fn calculates_changes_and_returns_active_if_non_empty() {
		let api = TestApiV1;
		let data_source = MockGovernedMapDataSource {
			changes: vec![
				("updated_key".into(), Some(vec![63].into())),
				("inserted_key".into(), Some(vec![1, 2, 3].into())),
				("deleted_key".into(), None),
			]
			.into(),
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

		let GovernedMapInherentDataProvider::ActiveV1 { changes } = idp else {
			panic!("IDP should be active")
		};

		let expected_changes = vec![
			GovernedMapChangeV1::delete("deleted_key"),
			GovernedMapChangeV1::upsert("inserted_key".into(), &[1, 2, 3]),
			GovernedMapChangeV1::upsert("updated_key", &[63]),
		];

		assert_eq!(changes, expected_changes);
	}

	#[tokio::test]
	async fn is_empty_when_there_are_no_changes() {
		let api = TestApiV1;
		let data_source = MockGovernedMapDataSource { changes: vec![] };

		let idp = GovernedMapInherentDataProvider::new(
			&api,
			<Block as BlockT>::Hash::default(),
			McBlockHash::default(),
			Some(McBlockHash::default()),
			&data_source,
		)
		.await
		.expect("Should succeed");

		assert_eq!(idp, GovernedMapInherentDataProvider::ActiveV1 { changes: vec![] });
	}

	#[tokio::test]
	async fn provides_inherent_data_when_non_empty() {
		let changes = vec![
			GovernedMapChangeV1::delete("deleted_key"),
			GovernedMapChangeV1::upsert("inserted_key", &[1, 2, 3]),
			GovernedMapChangeV1::upsert("updated_key", &[63]),
		];

		let idp = GovernedMapInherentDataProvider::ActiveV1 { changes: changes.clone() };

		let mut inherent_data = InherentData::new();
		idp.provide_inherent_data(&mut inherent_data).await.unwrap();

		assert_eq!(
			inherent_data
				.get_data::<ChangesV1>(&INHERENT_IDENTIFIER)
				.expect("Should succeed")
				.expect("Data should be present"),
			changes
		);
	}

	#[tokio::test]
	async fn does_not_provide_inherent_data_when_empty() {
		let idp = GovernedMapInherentDataProvider::ActiveV1 { changes: Vec::new() };

		let mut inherent_data = InherentData::new();
		idp.provide_inherent_data(&mut inherent_data).await.unwrap();

		assert!(inherent_data
			.get_data::<ChangesV1>(&INHERENT_IDENTIFIER)
			.expect("Should succeed")
			.is_none());
	}

	#[tokio::test]
	async fn does_not_provide_inherent_data_when_inert() {
		let idp = GovernedMapInherentDataProvider::Inert;

		let mut inherent_data = InherentData::new();
		idp.provide_inherent_data(&mut inherent_data).await.unwrap();

		assert!(inherent_data
			.get_data::<ChangesV1>(&INHERENT_IDENTIFIER)
			.expect("Should succeed")
			.is_none());
	}
}
