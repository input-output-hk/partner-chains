use crate::candidates::CandidatesDataSourceImpl;
use crate::db_model::index_exists;
use crate::metrics::mock::test_metrics;
use authority_selection_inherents::authority_selection_inputs::{
	AuthoritySelectionDataSource, RawPermissionedCandidateData,
};
use hex_literal::hex;
use sidechain_domain::*;
use sqlx::PgPool;
use std::str::FromStr;
use tokio_test::assert_err;

const D_PARAM_POLICY: [u8; 28] = hex!("500000000000000000000000000000000000434845434b504f494e69");
const PERMISSIONED_CANDIDATES_POLICY: [u8; 28] =
	hex!("500000000000000000000000000000000000434845434b504f494e19");

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_candidates_for_epoch(pool: PgPool) {
	let source = make_source(pool);
	let result = source.get_candidates(McEpochNumber(191), candidates_address()).await.unwrap();
	let mut candidates = result;
	candidates.sort_by(|c1, c2| c1.mainchain_pub_key().0.cmp(&c2.mainchain_pub_key().0));
	assert_eq!(candidates, vec![leader_candidate_spo_a(), leader_candidate_spo_b()])
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_candidates_after_some_deregistrations(pool: PgPool) {
	let source = make_source(pool);
	let result = source.get_candidates(McEpochNumber(195), candidates_address()).await.unwrap();
	let mut candidates = result;
	candidates.sort_by(|c1, c2| c1.mainchain_pub_key().0.cmp(&c2.mainchain_pub_key().0));
	assert_eq!(candidates, vec![leader_candidate_spo_c(), leader_candidate_spo_b()])
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_epoch_nonce(pool: PgPool) {
	let source = make_source(pool);
	let epoch_189_nonce = EpochNonce(
		hex!("ABEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1").to_vec(),
	);
	let result = source.get_epoch_nonce(McEpochNumber(191)).await.unwrap();
	assert_eq!(result, Some(epoch_189_nonce));
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_ariadne_parameters_returns_err_if_there_were_no_set_transactions(pool: PgPool) {
	let source = make_source(pool);
	// The first permissioned candidates tx was submitted at epoch 190
	let result = source
		.get_ariadne_parameters(
			McEpochNumber(3),
			d_parameter_policy(),
			permissioned_candidates_policy(),
		)
		.await;
	assert_err!(result);
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_ariadne_parameters_returns_the_latest_value_for_the_future_epochs(pool: PgPool) {
	let source = make_source(pool);
	// The last tx was submitted at epoch 192
	let result = source
		.get_ariadne_parameters(
			McEpochNumber(200),
			d_parameter_policy(),
			permissioned_candidates_policy(),
		)
		.await
		.unwrap();
	assert_eq!(
		result.d_parameter,
		DParameter { num_permissioned_candidates: 1, num_registered_candidates: 3 }
	)
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_ariadne_parameters_returns_the_latest_candidates_if_there_were_multiple_in_the_same_epoch(
	pool: PgPool,
) {
	let source = make_source(pool);
	// There were 2 transactions in epoch 190, one at block "1", second at block "2"
	// see 6_insert_transactions.sql
	let result = source
		.get_ariadne_parameters(
			McEpochNumber(192),
			d_parameter_policy(),
			permissioned_candidates_policy(),
		)
		.await
		.unwrap();
	assert_eq!(result.permissioned_candidates, Some(latest_permissioned_candidates()));
	assert_eq!(
		result.d_parameter,
		DParameter { num_permissioned_candidates: 1, num_registered_candidates: 3 }
	)
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_ariadne_parameters_returns_none_when_permissioned_list_not_set(pool: PgPool) {
	let source = make_source(pool);
	let result = source
		.get_ariadne_parameters(
			McEpochNumber(191),
			d_parameter_policy(),
			permissioned_candidates_policy(),
		)
		.await
		.unwrap();
	assert_eq!(
		result.d_parameter,
		DParameter { num_permissioned_candidates: 1, num_registered_candidates: 3 }
	);
	assert_eq!(result.permissioned_candidates, None)
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_ariadne_parameters_returns_the_latest_params_for_the_future_epochs(pool: PgPool) {
	let source = make_source(pool);
	// The last tx was submitted at epoch 192
	let result = source
		.get_ariadne_parameters(
			McEpochNumber(2000),
			d_parameter_policy(),
			permissioned_candidates_policy(),
		)
		.await
		.unwrap();
	assert_eq!(result.permissioned_candidates, Some(latest_permissioned_candidates()))
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_make_source_creates_index(pool: PgPool) {
	assert!(!index_exists(&pool, "idx_ma_tx_out_ident").await);
	CandidatesDataSourceImpl::new(pool.clone(), None).await.unwrap();
	assert!(index_exists(&pool, "idx_ma_tx_out_ident").await);
}

mod candidate_caching {
	use super::super::*;
	use crate::candidates::cached::CandidateDataSourceCached;
	use crate::candidates::tests::*;

	#[sqlx::test(migrations = "./testdata/migrations")]
	async fn candidates_caching_test(pool: PgPool) {
		let security_parameter = 2;
		let cache_size = 100;
		let service = CandidateDataSourceCached::new(
			make_source(pool.clone()),
			cache_size,
			security_parameter,
		);
		// With security parameter 2, block 3 (from epoch 191) is the latest stable block, so epoch 190 is the latest stable epoch.
		// get_candidates(192) uses data from the last block of epoch 190 (block 2), so result should be cached.
		let epoch_192_candidates =
			service.get_candidates(McEpochNumber(192), candidates_address()).await.unwrap();
		// get_candidates(193) uses data from the last block of epoch 191 (block 3), so result should not be cached.
		let epoch_193_candidates =
			service.get_candidates(McEpochNumber(193), candidates_address()).await.unwrap();
		assert!(!epoch_193_candidates.is_empty());
		// Remove all registrations to prove that one request was cached and the other not
		sqlx::raw_sql("DELETE FROM tx WHERE block_id >= 0")
			.execute(&pool)
			.await
			.unwrap();
		let epoch_192_candidates_after_txs_removal =
			service.get_candidates(McEpochNumber(192), candidates_address()).await.unwrap();
		assert_eq!(epoch_192_candidates, epoch_192_candidates_after_txs_removal);
		let epoch_193_candidates_after_txs_removal =
			service.get_candidates(McEpochNumber(193), candidates_address()).await.unwrap();
		// Proves epoch 193 candidates were not cached
		assert_ne!(epoch_193_candidates, epoch_193_candidates_after_txs_removal);
	}

	#[sqlx::test(migrations = "./testdata/migrations")]
	async fn candidates_caching_key_test(pool: PgPool) {
		let service = CandidateDataSourceCached::new(make_source(pool.clone()), 10, 2);
		// With security parameter 2, block 3 (from epoch 191) is the latest stable block, so epoch 190 is the latest stable epoch.
		// get_candidates(192) uses data from the last block of epoch 190 (block 2), so result should be cached.
		let epoch_192_candidates =
			service.get_candidates(McEpochNumber(192), candidates_address()).await.unwrap();
		let epoch_192_candidates_from_different_address = service
			.get_candidates(McEpochNumber(192), MainchainAddress::from_str("script_addr2").unwrap())
			.await
			.unwrap();
		assert!(!epoch_192_candidates.is_empty());
		assert!(epoch_192_candidates_from_different_address.is_empty());
	}

	#[sqlx::test(migrations = "./testdata/migrations")]
	async fn ariadne_parameters_caching_test(pool: PgPool) {
		let security_parameter = 2;
		let cache_size = 100;
		let service = CandidateDataSourceCached::new(
			make_source(pool.clone()),
			cache_size,
			security_parameter,
		);
		// With security parameter 2, block 3 (from epoch 191) is the latest stable block, so epoch 190 is the latest stable epoch.
		// get_ariadne_parameters(192) uses data from the last block of epoch 190 (block 2), so result should be cached.
		let epoch_192_ariadne_parameters = service
			.get_ariadne_parameters(
				McEpochNumber(192),
				d_parameter_policy(),
				permissioned_candidates_policy(),
			)
			.await
			.unwrap();
		// get_ariadne_parameters(193) uses data from the last block of epoch 191 (block 3), so result should not be cached.
		let epoch_193_ariadne_parameters = service
			.get_ariadne_parameters(
				McEpochNumber(193),
				d_parameter_policy(),
				permissioned_candidates_policy(),
			)
			.await
			.unwrap();
		assert_eq!(
			epoch_193_ariadne_parameters.d_parameter,
			DParameter { num_permissioned_candidates: 1, num_registered_candidates: 3 }
		);
		// Remove all registrations to prove that one request was cached and the other not
		sqlx::raw_sql("DELETE FROM tx WHERE block_id >= 0")
			.execute(&pool)
			.await
			.unwrap();
		let epoch_192_ariadne_parameters_after_tx_removal = service
			.get_ariadne_parameters(
				McEpochNumber(192),
				d_parameter_policy(),
				permissioned_candidates_policy(),
			)
			.await
			.unwrap();
		assert_eq!(epoch_192_ariadne_parameters, epoch_192_ariadne_parameters_after_tx_removal);
		let epoch_193_ariadne_parameters_after_txs_removal = service
			.get_ariadne_parameters(
				McEpochNumber(193),
				d_parameter_policy(),
				permissioned_candidates_policy(),
			)
			.await;
		// Proves epoch 193 candidates were not cached
		assert!(epoch_193_ariadne_parameters_after_txs_removal.is_err());
	}

	#[sqlx::test(migrations = "./testdata/migrations")]
	async fn ariadne_parameters_caching_key_test(pool: PgPool) {
		let service = CandidateDataSourceCached::new(make_source(pool.clone()), 10, 2);
		// With security parameter 2, block 3 (from epoch 191) is the latest stable block, so epoch 190 is the latest stable epoch.
		// get_ariadne_parameters(192) uses data from the last block of epoch 190 (block 2), so result should be cached.
		let epoch_192_ariadne_parameters_result = service
			.get_ariadne_parameters(
				McEpochNumber(192),
				d_parameter_policy(),
				permissioned_candidates_policy(),
			)
			.await;
		let epoch_192_ariadne_parameters_for_different_policy_result = service
			.get_ariadne_parameters(
				McEpochNumber(192),
				PolicyId(hex!("aabb00000000000000000000000000000000434845434b504f494e69")),
				permissioned_candidates_policy(),
			)
			.await;
		assert!(epoch_192_ariadne_parameters_result.is_ok());
		assert!(epoch_192_ariadne_parameters_for_different_policy_result.is_err());
	}
}

fn make_source(pool: PgPool) -> CandidatesDataSourceImpl {
	CandidatesDataSourceImpl { pool, metrics_opt: Some(test_metrics()) }
}

fn candidates_address() -> MainchainAddress {
	MainchainAddress::from_str("script_addr").unwrap()
}

fn d_parameter_policy() -> PolicyId {
	PolicyId(D_PARAM_POLICY)
}

fn permissioned_candidates_policy() -> PolicyId {
	PolicyId(PERMISSIONED_CANDIDATES_POLICY)
}

fn latest_permissioned_candidates() -> Vec<RawPermissionedCandidateData> {
	vec![
		RawPermissionedCandidateData {
			sidechain_public_key: SidechainPublicKey(
				hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854").to_vec(),
			),
			aura_public_key: AuraPublicKey(
				hex!("bf20afa1c1a72af3341fa7a447e3f9eada9f3d054a7408fb9e49ad4d6e6559ec").to_vec(),
			),
			grandpa_public_key: GrandpaPublicKey(
				hex!("9042a40b0b1baa9adcead024432a923eac706be5e1a89d7f2f2d58bfa8f3c26d").to_vec(),
			),
		},
		RawPermissionedCandidateData {
			sidechain_public_key: SidechainPublicKey(
				hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf").to_vec(),
			),
			aura_public_key: AuraPublicKey(
				hex!("56d1da82e56e4cb35b13de25f69a3e9db917f3e13d6f786321f4b0a9dc153b19").to_vec(),
			),
			grandpa_public_key: GrandpaPublicKey(
				hex!("7392f3ea668aa2be7997d82c07bcfbec3ee4a9a4e01e3216d92b8f0d0a086c32").to_vec(),
			),
		},
	]
}

fn leader_candidate_spo_a() -> CandidateRegistrations {
	CandidateRegistrations {
			stake_pool_public_key: StakePoolPublicKey(hex!("bfbee74ab533f40979101057f96de62e95233f2a5216eb16b54106f09fd7350d")),
			registrations: vec![
				RegistrationData {
					registration_utxo: UtxoId::new(hex!("cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"), 1),
					sidechain_signature: SidechainSignature(hex!("f8ec6c7f935d387aaa1693b3bf338cbb8f53013da8a5a234f9c488bacac01af259297e69aee0df27f553c0a1164df827d016125c16af93c99be2c19f36d2f66e").to_vec()),
					mainchain_signature: MainchainSignature(hex!("28d1c3b7df297a60d24a3f88bc53d7029a8af35e8dd876764fd9e7a24203a3482a98263cc8ba2ddc7dc8e7faea31c2e7bad1f00e28c43bc863503e3172dc6b0a")),
					cross_chain_signature: CrossChainSignature(hex!("f8ec6c7f935d387aaa1693b3bf338cbb8f53013da8a5a234f9c488bacac01af259297e69aee0df27f553c0a1164df827d016125c16af93c99be2c19f36d2f66e").to_vec()),
					sidechain_pub_key: SidechainPublicKey(hex!("02fe8d1eb1bcb3432b1db5833ff5f2226d9cb5e65cee430558c18ed3a3c86ce1af").to_vec()),
					cross_chain_pub_key: CrossChainPublicKey(hex!("02fe8d1eb1bcb3432b1db5833ff5f2226d9cb5e65cee430558c18ed3a3c86ce1af").to_vec()),
					aura_pub_key: AuraPublicKey(hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d").to_vec()),
					grandpa_pub_key: GrandpaPublicKey(hex!("88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee").to_vec()),
					utxo_info: UtxoInfo {
						utxo_id: UtxoId::new(hex!("abeed7fb0067f14d6f6436c7f7dedb27ce3ceb4d2d18ff249d43b22d86fae3f1"), 0),
						epoch_number: McEpochNumber(189),
						block_number: McBlockNumber(0),
						slot_number: McSlotNumber(189410),
						tx_index_within_block: McTxIndexInBlock(1),
					},
					tx_inputs: vec![
						UtxoId::new(hex!("cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"), 1),
						UtxoId::new(hex!("cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"), 0),
					],
				}
			],
			stake_delegation: Some(StakeDelegation(1001995478725)),
		}
}

fn leader_candidate_spo_b() -> CandidateRegistrations {
	CandidateRegistrations {
			stake_pool_public_key: StakePoolPublicKey(hex!("cfbee74ab533f40979101057f96de62e95233f2a5216eb16b54106f09fd7350d")),
			registrations: vec![
				RegistrationData {
					registration_utxo: UtxoId::new(hex!("cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"), 1),
					sidechain_signature: SidechainSignature(hex!("f8ec6c7f935d387aaa1693b3bf338cbb8f53013da8a5a234f9c488bacac01af259297e69aee0df27f553c0a1164df827d016125c16af93c99be2c19f36d2f66e").to_vec()),
					mainchain_signature: MainchainSignature(hex!("28d1c3b7df297a60d24a3f88bc53d7029a8af35e8dd876764fd9e7a24203a3482a98263cc8ba2ddc7dc8e7faea31c2e7bad1f00e28c43bc863503e3172dc6b0a")),
					cross_chain_signature: CrossChainSignature(hex!("f8ec6c7f935d387aaa1693b3bf338cbb8f53013da8a5a234f9c488bacac01af259297e69aee0df27f553c0a1164df827d016125c16af93c99be2c19f36d2f66e").to_vec()),
					sidechain_pub_key: SidechainPublicKey(hex!("02fe8d1eb1bcb3432b1db5833ff5f2226d9cb5e65cee430558c18ed3a3c86ce1af").to_vec()),
					cross_chain_pub_key: CrossChainPublicKey(hex!("02fe8d1eb1bcb3432b1db5833ff5f2226d9cb5e65cee430558c18ed3a3c86ce1af").to_vec()),
					aura_pub_key: AuraPublicKey(hex!("8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48").to_vec()),
					grandpa_pub_key: GrandpaPublicKey(hex!("d17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69").to_vec()),
					utxo_info: UtxoInfo {
						utxo_id: UtxoId::new(hex!("abeed7fb0067f14d6f6436c7f7dedb27ce3ceb4d2d18ff249d43b22d86fae3f1"), 2),
						epoch_number: McEpochNumber(189),
						block_number: McBlockNumber(0),
						slot_number: McSlotNumber(189410),
						tx_index_within_block: McTxIndexInBlock(1),
					},
					tx_inputs: vec![
						UtxoId::new(hex!("cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"), 1),
						UtxoId::new(hex!("cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"), 0),
					],
				}
			],
			stake_delegation: Some(StakeDelegation(5001995651486)),
		}
}

fn leader_candidate_spo_c() -> CandidateRegistrations {
	CandidateRegistrations {
			stake_pool_public_key: StakePoolPublicKey(hex!("3fd6618bfcb8d964f44beba4280bd91c6e87ac5bca4aa1c8f1cde9e85352660b")),
			registrations: vec![
				RegistrationData {
					registration_utxo: UtxoId::new(hex!("cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"), 2),
					sidechain_signature: SidechainSignature(hex!("3e8a8b29e513a08d0a66e22422a1a85d1bf409987f30a8c6fcab85ba38a85d0d27793df7e7fb63ace12203b062feb7edb5e6664ac1810b94c38182acc6167425").to_vec()),
					mainchain_signature: MainchainSignature(hex!("1fd2f1e5ad14c829c7359474764701cd74ab9c433c29b0bbafaa6bcf22376e9d651391d08ae6f40b418d2abf827c4c1fcb007e779a2beba7894d68012942c708")),
					cross_chain_signature: CrossChainSignature(hex!("3e8a8b29e513a08d0a66e22422a1a85d1bf409987f30a8c6fcab85ba38a85d0d27793df7e7fb63ace12203b062feb7edb5e6664ac1810b94c38182acc6167425").to_vec()),
					sidechain_pub_key: SidechainPublicKey(hex!("02333e47cab242fefe88d7da1caa713307290291897f100efb911672d317147f72").to_vec()),
					cross_chain_pub_key: CrossChainPublicKey(hex!("02333e47cab242fefe88d7da1caa713307290291897f100efb911672d317147f72").to_vec()),
					aura_pub_key: AuraPublicKey(hex!("8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f23333").to_vec()),
					grandpa_pub_key: GrandpaPublicKey(hex!("d17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fad3333").to_vec()),
					utxo_info: UtxoInfo {
						utxo_id: UtxoId::new(hex!("055557fb0067f14d6f6436c7f7dedb27ce3ceb4d2d18ff249d43b22d86fae3f1"), 0),
						epoch_number: McEpochNumber(191),
						block_number: McBlockNumber(3),
						slot_number: McSlotNumber(191500),
						tx_index_within_block: McTxIndexInBlock(0),
					},
					tx_inputs: vec![
						UtxoId::new(hex!("cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"), 2),
					],
				}
			],
			stake_delegation: Some(StakeDelegation(123456789)),
		}
}
