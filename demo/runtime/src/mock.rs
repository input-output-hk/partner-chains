use crate::*;
use authority_selection_inherents::{
	AriadneInherentDataProvider, AuthoritySelectionInputs, RegisterValidatorSignedMessage,
};
use frame_support::{Hashable, pallet_prelude::*};
use hex_literal::hex;
use pallet_session_validator_management::CommitteeMemberOf;
use plutus::ToDatum;
use sidechain_domain::*;
use sp_core::crypto::CryptoType;
use sp_core::{ByteArray, Pair, crypto::AccountId32, ed25519};
use sp_runtime::{BuildStorage, Digest, DigestItem, MultiSigner, traits::IdentifyAccount};
use sp_std::vec::Vec;
use std::cmp::max;

const DUMMY_EPOCH_NONCE: &[u8] = &[1u8, 2u8, 3u8];

use sp_consensus_aura::AURA_ENGINE_ID;

// Build genesis storage
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

	let initial_authorities: Vec<_> = vec![alice().as_permissioned(), bob().as_permissioned()];

	let session_keys: Vec<_> =
		vec![(alice().account(), alice().session()), (bob().account(), bob().session())];
	let main_chain_scripts = MainChainScripts::default();

	pallet_sidechain::GenesisConfig::<Runtime> {
		genesis_utxo: UtxoId::new(
			hex!("abcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd"),
			0,
		),
		epoch_duration: ScEpochDuration::from_millis(360_000),
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	pallet_session_validator_management::GenesisConfig::<Runtime> {
		initial_authorities,
		main_chain_scripts,
	}
	.assimilate_storage(&mut t)
	.unwrap();

	pallet_session::GenesisConfig::<Runtime> {
		keys: session_keys
			.into_iter()
			.map(|(cross_chain, session)| (cross_chain.clone().into(), cross_chain.into(), session))
			.collect(),
		non_authority_keys: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}

pub fn slots_to_epoch(epoch: u64, slots_per_epoch: u32) -> u64 {
	let epoch = ARBITRARY_FIRST_EPOCH + epoch;
	let current_slot = pallet_aura::CurrentSlot::<Runtime>::get();
	let to_slot = epoch * (slots_per_epoch as u64);
	to_slot - *current_slot
}

pub fn advance_block() {
	finalize_block();
	initialize_block();
}

pub const SLOTS_PER_EPOCH: u32 = 60;
// in real life first slot will be something much bigger than 0, that's why it is here
pub const ARBITRARY_FIRST_SLOT: u64 = 389374234;
pub const ARBITRARY_FIRST_EPOCH: u64 = ARBITRARY_FIRST_SLOT / (SLOTS_PER_EPOCH as u64);

pub fn initialize_block() {
	let slot = *pallet_aura::CurrentSlot::<Runtime>::get() + 1;
	let slot = if slot == 1 { slot + ARBITRARY_FIRST_SLOT } else { slot };
	initialize_with_slot_digest_and_increment_block_number(slot);

	System::on_initialize(System::block_number());
	Aura::on_initialize(System::block_number());
	Grandpa::on_initialize(System::block_number());
	SessionCommitteeManagement::on_initialize(System::block_number());
	Session::on_initialize(System::block_number());

	let block_number = System::block_number();
	let epoch = Sidechain::current_epoch_number();
	assert_eq!(slot, *pallet_aura::CurrentSlot::<Runtime>::get());
	println!("(slot {slot}, epoch {epoch}) Initialized block {block_number}");
}

pub fn finalize_block() {
	if System::block_number() > 0 {
		Session::on_finalize(System::block_number());
		SessionCommitteeManagement::on_finalize(System::block_number());
		Grandpa::on_finalize(System::block_number());
		Aura::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
	}
}

pub fn for_next_n_blocks(n: u32, f: &dyn Fn()) {
	for _ in 1..=n {
		f();
		advance_block();
	}
}

pub fn until_epoch(epoch: u64, f: &dyn Fn()) {
	let n = slots_to_epoch(epoch, SLOTS_PER_EPOCH);
	for_next_n_blocks(u32::try_from(n).unwrap(), f)
}

pub fn until_epoch_after_finalizing(epoch: u64, f: &dyn Fn()) {
	let n = slots_to_epoch(epoch, SLOTS_PER_EPOCH);
	for_next_n_blocks_after_finalizing(u32::try_from(n).unwrap(), f)
}

pub fn for_next_n_blocks_after_finalizing(n: u32, f: &dyn Fn()) {
	for _ in 1..=n {
		finalize_block();
		f();
		initialize_block()
	}
}

pub fn create_inherent_data_struct(
	expected_authorities: &[TestKeys],
) -> AriadneInherentDataProvider {
	let genesis_utxo = pallet_sidechain::Pallet::<Runtime>::genesis_utxo();

	let candidates: Vec<CandidateRegistrations> = expected_authorities
		.iter()
		.map(|validator| {
			let validator_pub_key: [u8; 33] =
				validator.cross_chain.public().to_raw_vec().try_into().unwrap();
			let mainchain_pub_key_seed: [u8; 32] = validator_pub_key.blake2_256();
			let dummy_mainchain_pub_key: ed25519::Pair = Pair::from_seed(&mainchain_pub_key_seed);
			let signed_message = RegisterValidatorSignedMessage {
				genesis_utxo,
				sidechain_pub_key: validator.cross_chain.public().into_inner().0.to_vec(),
				registration_utxo: UtxoId::default(),
			};

			let signed_message_encoded = minicbor::to_vec(signed_message.to_datum()).unwrap();

			let mainchain_signature = dummy_mainchain_pub_key.sign(&signed_message_encoded[..]);
			let sidechain_signature = validator.cross_chain.sign(&signed_message_encoded[..]);

			let registration_data = RegistrationData {
				registration_utxo: signed_message.registration_utxo,
				sidechain_signature: SidechainSignature(
					sidechain_signature.into_inner().0[..64].to_vec(),
				),
				mainchain_signature: MainchainSignature(mainchain_signature.0),
				cross_chain_signature: CrossChainSignature(vec![]),
				sidechain_pub_key: SidechainPublicKey(
					validator.cross_chain.public().into_inner().0.to_vec(),
				),
				cross_chain_pub_key: CrossChainPublicKey(vec![]),
				keys: validator.candidate_keys(),
				utxo_info: UtxoInfo::default(),
				tx_inputs: vec![signed_message.registration_utxo],
			};

			CandidateRegistrations {
				stake_pool_public_key: StakePoolPublicKey(dummy_mainchain_pub_key.public().0),
				registrations: vec![registration_data],
				stake_delegation: Some(StakeDelegation(7)),
			}
		})
		.collect();

	AriadneInherentDataProvider {
		data: Some(AuthoritySelectionInputs {
			d_parameter: DParameter {
				num_permissioned_candidates: 0,
				num_registered_candidates: max(candidates.len() as u16, 1),
			},
			permissioned_candidates: vec![],
			registered_candidates: candidates,
			epoch_nonce: EpochNonce(DUMMY_EPOCH_NONCE.to_vec()),
		}),
	}
}

pub type CrossChainPair = <CrossChainPublic as CryptoType>::Pair;

const ALICE_SEED: &str = "//1";
const BOB_SEED: &str = "//2";

#[derive(Clone)]
pub struct TestKeys {
	pub cross_chain: CrossChainPair,
	pub aura: sp_consensus_aura::sr25519::AuthorityPair,
	pub grandpa: sp_consensus_grandpa::AuthorityPair,
}

impl TestKeys {
	pub fn from_seed(s: &str) -> Self {
		Self { cross_chain: pair_from_seed(s), aura: pair_from_seed(s), grandpa: pair_from_seed(s) }
	}
	pub fn account(&self) -> AccountId32 {
		MultiSigner::from(sp_core::ecdsa::Public::from(self.cross_chain.public())).into_account()
	}
	pub fn session(&self) -> SessionKeys {
		SessionKeys { aura: self.aura.public(), grandpa: self.grandpa.public() }
	}
	pub fn candidate_keys(&self) -> CandidateKeys {
		CandidateKeys(
			self.session()
				.into_raw_public_keys()
				.into_iter()
				.map(|(value, key_type_id)| CandidateKey::new(key_type_id, value))
				.collect(),
		)
	}
	pub fn as_permissioned(&self) -> CommitteeMemberOf<Runtime> {
		CommitteeMember::permissioned(self.cross_chain.public(), self.session())
	}
}

pub fn pair_from_seed<P: Pair>(seed: &str) -> P {
	<P as Pair>::from_string(seed, None).expect("static values are valid; qed")
}

pub fn alice() -> TestKeys {
	TestKeys::from_seed(ALICE_SEED)
}

pub fn bob() -> TestKeys {
	TestKeys::from_seed(BOB_SEED)
}

fn initialize_with_slot_digest_and_increment_block_number(slot_number: u64) {
	let slot = sp_consensus_slots::Slot::from(slot_number);
	let pre_digest = Digest { logs: vec![DigestItem::PreRuntime(AURA_ENGINE_ID, slot.encode())] };

	System::reset_events();
	System::initialize(&(System::block_number() + 1), &System::parent_hash(), &pre_digest);
}

macro_rules! assert_current_epoch {
	($epoch:expr) => {{
		assert_eq!(Sidechain::current_epoch_number().0, $epoch + ARBITRARY_FIRST_EPOCH);
	}};
}
pub(crate) use assert_current_epoch;

macro_rules! assert_next_committee {
	([$($member:expr),*]) => {{
		let next = SessionCommitteeManagement::next_committee().unwrap().into_inner();
		assert_eq!(next, vec![$($member.cross_chain.public()),*])
	}};
}
pub(crate) use assert_next_committee;

macro_rules! assert_grandpa_authorities {
	([$($member:expr),*]) => {{
		let expected_authorities = HashSet::from([$($member.grandpa.public()),*]);
		let actual_authorities: Vec<sp_consensus_grandpa::AuthorityId> = Grandpa::grandpa_authorities()
			.into_iter()
			.map(|(authority_id, _)| authority_id)
			.collect();
		let actual_authorities = HashSet::<_>::from_iter(actual_authorities);
		assert_eq!(actual_authorities, expected_authorities);
	}};
}
pub(crate) use assert_grandpa_authorities;

macro_rules! assert_aura_authorities {
    ([$($member:expr),*]) => {{
		let expected_authorities = vec![$($member.aura.public()),*];
		let actual_authorities = pallet_aura::Authorities::<Runtime>::get();
		assert_eq!(actual_authorities, expected_authorities);
	}};
}
pub(crate) use assert_aura_authorities;
use sp_session_validator_management::{CommitteeMember, MainChainScripts};

use crate::{CrossChainPublic, opaque::SessionKeys};
