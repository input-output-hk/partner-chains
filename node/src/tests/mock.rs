#![allow(clippy::type_complexity)]

use crate::inherent_data::CreateInherentDataConfig;
use crate::main_chain_follower::DataSources;
use crate::tests::runtime_api_mock::TestApi;
use authority_selection_inherents::filter_invalid_candidates::RegisterValidatorSignedMessage;
use chain_params::SidechainParams;
use epoch_derivation::{EpochConfig, MainchainEpochConfig};
use hex_literal::hex;
use main_chain_follower_api::mock_services::TestDataSources;
use plutus::ToDatum;
use sc_consensus_aura::SlotDuration;
use sidechain_domain::*;
use sidechain_slots::{ScSlotConfig, SlotsPerEpoch};
use sp_core::offchain::{Duration, Timestamp};
use sp_core::{ecdsa, ed25519, Pair};
use std::sync::Arc;

impl From<TestDataSources> for DataSources {
	fn from(value: TestDataSources) -> Self {
		Self { block: value.block, candidate: value.candidate, native_token: value.native_token }
	}
}

pub fn mock_sidechain_params() -> SidechainParams {
	SidechainParams {
		chain_id: 0,
		genesis_committee_utxo: UtxoId {
			tx_hash: McTxHash(hex!(
				"f17e6d3aa72095e04489d13d776bf05a66b5a8c49d89397c28b18a1784b9950e"
			)),
			index: UtxoIndex(0),
		},
		threshold_numerator: 2,
		threshold_denominator: 3,
		governance_authority: MainchainAddressHash(hex!(
			"00112233445566778899001122334455667788990011223344556677"
		)),
	}
}

pub fn test_slot_config() -> ScSlotConfig {
	ScSlotConfig {
		slots_per_epoch: SlotsPerEpoch(10),
		slot_duration: SlotDuration::from_millis(1000),
	}
}

pub fn test_epoch_config() -> EpochConfig {
	let sc_slot_config = test_slot_config();
	EpochConfig {
		mc: MainchainEpochConfig {
			first_epoch_timestamp_millis: Timestamp::from_unix_millis(0),
			first_epoch_number: 0,
			epoch_duration_millis: Duration::from_millis(
				u64::from(sc_slot_config.slots_per_epoch.0)
					* sc_slot_config.slot_duration.as_millis()
					* 10,
			),
			first_slot_number: 0,
		},
	}
}

pub fn test_client() -> Arc<TestApi> {
	Arc::new(TestApi::new(ScEpochNumber(2)))
}

pub fn test_create_inherent_data_config() -> CreateInherentDataConfig {
	CreateInherentDataConfig {
		epoch_config: test_epoch_config(),
		sc_slot_config: test_slot_config(),
		time_source: Arc::new(time_source::MockedTimeSource { current_time_millis: 30000 }),
	}
}

pub fn create_candidates(
	candidates_info: Vec<([u8; 32], [u8; 33], ([u8; 32], [u8; 32]))>,
) -> Vec<CandidateRegistrations> {
	candidates_info
		.into_iter()
		.map(|(account_seed, crosschain_public, session_keys)| {
			let mainchain_account = ed25519::Pair::from_seed_slice(&account_seed).unwrap();
			let sidechain_account = ecdsa::Pair::from_seed_slice(&account_seed).unwrap();
			let sidechain_params = mock_sidechain_params();

			let registration_data = create_valid_registration_data(
				mainchain_account,
				sidechain_account,
				crosschain_public,
				session_keys,
				sidechain_params,
			);

			CandidateRegistrations {
				mainchain_pub_key: MainchainPublicKey(mainchain_account.public().0),
				registrations: vec![registration_data],
				stake_delegation: Some(StakeDelegation(7)),
			}
		})
		.collect()
}

pub fn create_valid_registration_data(
	mainchain_account: ed25519::Pair,
	sidechain_account: ecdsa::Pair,
	crosschain_public: [u8; 33],
	session_keys: ([u8; 32], [u8; 32]),
	sidechain_params: SidechainParams,
) -> RegistrationData {
	let signed_message = RegisterValidatorSignedMessage {
		sidechain_params,
		sidechain_pub_key: sidechain_account.public().0.to_vec(),
		input_utxo: UtxoId { tx_hash: McTxHash([7u8; TX_HASH_SIZE]), index: UtxoIndex(0) },
	};

	let signed_message_encoded = minicbor::to_vec(signed_message.to_datum()).unwrap();

	let mainchain_signature = mainchain_account.sign(&signed_message_encoded[..]);
	let sidechain_signature = sidechain_account.sign(&signed_message_encoded[..]);

	RegistrationData {
		consumed_input: signed_message.input_utxo,
		// Specification requires the signature length to be 64 instead of 65
		sidechain_signature: SidechainSignature(sidechain_signature.0[0..64].to_vec()),
		mainchain_signature: MainchainSignature(mainchain_signature.0.to_vec()),
		cross_chain_signature: CrossChainSignature(vec![]),
		sidechain_pub_key: SidechainPublicKey(sidechain_account.public().0.to_vec()),
		cross_chain_pub_key: CrossChainPublicKey(crosschain_public.to_vec()),
		aura_pub_key: AuraPublicKey(session_keys.0.to_vec()),
		grandpa_pub_key: GrandpaPublicKey(session_keys.1.to_vec()),
		utxo_info: UtxoInfo {
			utxo_id: UtxoId { tx_hash: McTxHash([7u8; 32]), index: UtxoIndex(7) },
			epoch_number: McEpochNumber(7),
			block_number: McBlockNumber(7),
			slot_number: McSlotNumber(7),
			tx_index_within_block: McTxIndexInBlock(7),
		},
		tx_inputs: vec![signed_message.input_utxo],
	}
}
