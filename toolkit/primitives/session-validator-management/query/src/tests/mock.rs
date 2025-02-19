use authority_selection_inherents::filter_invalid_candidates::RegisterValidatorSignedMessage;
use plutus::ToDatum;
use sidechain_domain::*;
use sp_core::{ecdsa, ed25519, Pair};

pub(crate) const TEST_UTXO_ID: UtxoId =
	UtxoId { tx_hash: McTxHash([0u8; 32]), index: UtxoIndex(0) };

pub fn create_candidates(
	seeds: Vec<[u8; 32]>,
	genesis_utxo: UtxoId,
) -> Vec<CandidateRegistrations> {
	seeds
		.into_iter()
		.map(|s| create_valid_candidate_registrations_from_seed(s, genesis_utxo))
		.collect()
}

pub fn create_valid_candidate_registrations_from_seed(
	seed: [u8; 32],
	genesis_utxo: UtxoId,
) -> CandidateRegistrations {
	let mainchain_account = ed25519::Pair::from_seed_slice(&seed).unwrap();
	let sidechain_account = ecdsa::Pair::from_seed_slice(&seed).unwrap();

	let registration_data =
		create_valid_registration_data(mainchain_account, sidechain_account, genesis_utxo);

	CandidateRegistrations {
		stake_pool_public_key: StakePoolPublicKey(mainchain_account.public().0),
		registrations: vec![registration_data],
		stake_delegation: Some(StakeDelegation(7)),
	}
}

pub fn create_valid_registration_data(
	mainchain_account: ed25519::Pair,
	sidechain_account: ecdsa::Pair,
	genesis_utxo: UtxoId,
) -> RegistrationData {
	let signed_message = RegisterValidatorSignedMessage {
		genesis_utxo,
		sidechain_pub_key: sidechain_account.public().0.to_vec(),
		registration_utxo: UtxoId { tx_hash: McTxHash([7u8; TX_HASH_SIZE]), index: UtxoIndex(0) },
	};

	let signed_message_encoded = minicbor::to_vec(signed_message.to_datum()).unwrap();

	let mainchain_signature = mainchain_account.sign(&signed_message_encoded[..]);
	let sidechain_signature = sidechain_account.sign(&signed_message_encoded[..]);

	RegistrationData {
		registration_utxo: signed_message.registration_utxo,
		// Specification requires the signature length to be 64 instead of 65
		sidechain_signature: SidechainSignature(sidechain_signature.0[0..64].to_vec()),
		mainchain_signature: MainchainSignature(
			mainchain_signature.0.try_into().expect("Mainchain signature is 64 bytes"),
		),
		cross_chain_signature: CrossChainSignature(vec![]),
		sidechain_pub_key: SidechainPublicKey(sidechain_account.public().0.to_vec()),
		cross_chain_pub_key: CrossChainPublicKey(vec![]),
		aura_pub_key: AuraPublicKey(vec![1; 32]),
		grandpa_pub_key: GrandpaPublicKey(vec![3; 32]),
		utxo_info: UtxoInfo {
			utxo_id: UtxoId { tx_hash: McTxHash([7u8; 32]), index: UtxoIndex(7) },
			epoch_number: McEpochNumber(7),
			block_number: McBlockNumber(7),
			slot_number: McSlotNumber(7),
			tx_index_within_block: McTxIndexInBlock(7),
		},
		tx_inputs: vec![signed_message.registration_utxo],
	}
}
