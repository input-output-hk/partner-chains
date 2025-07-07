use crate::MaybeFromCandidateKeys;
use crate::authority_selection_inputs::AuthoritySelectionInputs;
use crate::filter_invalid_candidates::RegisterValidatorSignedMessage;
use crate::select_authorities::select_authorities;
use hex_literal::hex;
use num_bigint::BigInt;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use plutus::Datum::{ByteStringDatum, ConstructorDatum, IntegerDatum};
use plutus::ToDatum;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sidechain_domain::*;
use sp_core::{Pair, ecdsa, ed25519, sr25519};
use sp_runtime::key_types::{AURA, GRANDPA};
use sp_runtime::traits::Zero;

#[test]
fn registration_message_encoding() {
	//Expected datum and cbor hex were obtained using partner-chains-smart-contracts tests vectors
	//and are deemed as the source of truth.
	//Test data is at https://github.com/input-output-hk/partner-chains-smart-contracts/blob/54e561d62732e37d8f3b6f9e7c02d343122a5d4c/onchain/test/Test/TrustlessSidechain/Types.hs

	let sample_utxo_id_hash_bytes: [u8; 32] =
		hex!("e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efa");
	let sidechain_pub_key_bytes =
		hex!("02dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ec").to_vec();
	let genesis_utxo = UtxoId { tx_hash: McTxHash(sample_utxo_id_hash_bytes), index: UtxoIndex(4) };
	let msg = RegisterValidatorSignedMessage {
		genesis_utxo,
		sidechain_pub_key: sidechain_pub_key_bytes.clone(),
		// Unfortunately test vector in partner-chains-smart-contracts uses same UTxO in two places.
		registration_utxo: genesis_utxo,
	};

	let pub_key_datum = ByteStringDatum(sidechain_pub_key_bytes);
	let utxo_datum = ConstructorDatum {
		constructor: 0,
		fields: vec![
			ConstructorDatum {
				constructor: 0,
				fields: vec![ByteStringDatum(sample_utxo_id_hash_bytes.to_vec())],
			},
			IntegerDatum(BigInt::from(4)),
		],
	};
	let expected = ConstructorDatum {
		constructor: 0,
		fields: vec![utxo_datum.clone(), pub_key_datum, utxo_datum],
	};
	assert_eq!(msg.to_datum(), expected);

	let cbor_bytes = minicbor::to_vec(msg.to_datum()).unwrap();
	// https://github.com/input-output-hk/partner-chains-smart-contracts/blob/6e6aca0edeb09cecd3a93913020e9ceaa1ce1d25/onchain/test/golden/BlockProducerRegistrationMsg-cbor.golden#L1
	let expected_hex = "d8799fd8799fd8799f5820e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efaff04ff582102dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ecd8799fd8799f5820e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efaff04ffff";
	assert_eq!(hex::encode(cbor_bytes), expected_hex);
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AccountId(ecdsa::Public);

impl From<ecdsa::Public> for AccountId {
	fn from(value: ecdsa::Public) -> Self {
		Self(value)
	}
}

impl TryFrom<SidechainPublicKey> for AccountId {
	type Error = String;
	fn try_from(pk: SidechainPublicKey) -> Result<Self, String> {
		let bytes: [u8; 33] =
			pk.0.clone()
				.try_into()
				.map_err(|_| format!("{} is invalid ECDSA public key", hex::encode(pk.0)))?;
		Ok(AccountId(<ecdsa::Pair as Pair>::Public::from_raw(bytes)))
	}
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Encode,
	Decode,
	TypeInfo,
	MaxEncodedLen,
	Serialize,
	Deserialize,
)]
pub struct AccountKeys {
	pub aura: [u8; 32],
	pub grandpa: [u8; 32],
}

impl From<(sr25519::Public, ed25519::Public)> for AccountKeys {
	fn from((aura, grandpa): (sr25519::Public, ed25519::Public)) -> Self {
		Self { aura: aura.0, grandpa: grandpa.0 }
	}
}

impl AccountKeys {
	pub fn from_seed(seed: &str) -> AccountKeys {
		let mut aura = format!("aura-{seed}").into_bytes();
		aura.resize(32, 0);
		let mut grandpa = format!("grandpa-{seed}").into_bytes();
		grandpa.resize(32, 0);
		AccountKeys { aura: aura.try_into().unwrap(), grandpa: grandpa.try_into().unwrap() }
	}
}

pub struct TestConvertKeys;

impl MaybeFromCandidateKeys<AccountKeys> for TestConvertKeys {
	fn maybe_from(keys: &CandidateKeys) -> Option<AccountKeys> {
		let aura = <[u8; 32]>::try_from(keys.find_or_empty(AURA)).ok()?;
		let grandpa = <[u8; 32]>::try_from(keys.find_or_empty(GRANDPA)).ok()?;
		Some(AccountKeys { aura, grandpa })
	}
}

#[derive(Clone)]
pub(crate) struct MockValidator {
	pub name: &'static str,
	pub seed: &'static str,
	pub stake: u64,
}

pub const ALICE: MockValidator = MockValidator::new("alice", "//1", 100);
pub const BOB: MockValidator = MockValidator::new("bob", "//2", 200);
pub const CHARLIE: MockValidator = MockValidator::new("charlie", "//3", 300);
pub const DAVE: MockValidator = MockValidator::new("dave", "//4", 400);
pub const EVE: MockValidator = MockValidator::new("eve", "//5", 500);
pub const FERDIE: MockValidator = MockValidator::new("ferdie", "//6", 600);
pub const GREG: MockValidator = MockValidator::new("greg", "//7", 700);
pub const HENRY: MockValidator = MockValidator::new("henry", "//8", 800);
pub const IDA: MockValidator = MockValidator::new("ida", "//9", 900);
pub const JAMES: MockValidator = MockValidator::new("james", "//10", 1000);
pub const KIM: MockValidator = MockValidator::new("kim", "//11", 1100);

// Table for AccountId lookup, to improve `account_id_to_name` performance.
const ALL_MOCK_VALIDATORS: [(MockValidator, [u8; 33]); 11] = [
	(ALICE, hex!("0333022898140662dfea847e3cbfe5e989845ac6766e83472f8b0c650d85e77bae")),
	(BOB, hex!("02182879ec92e811e2a8cc117f3cde1f61d3cba0093134cfb1ed17a4ef74915d4a")),
	(CHARLIE, hex!("02f4f4d0eccb899bf2d611b56e0afec7c740efba404f8d0e82a545f988c45316c4")),
	(DAVE, hex!("03a0af06322d100056125fac1df39d161089b07ae279505aae8731c4d110a54ad7")),
	(EVE, hex!("03f045328f504c13dac9ddd9b1186098aee7c46cb8d55289dbbf2433bab7a26239")),
	(FERDIE, hex!("0325fc2095902f5fe394f244bce38b0dc3d631cbc05f0b64d5620a71bbf2514f0f")),
	(GREG, hex!("029a1eb2e31dcaf468dbb516f9b620fdd7c3f090d58a88e02b51b25255b2182dd1")),
	(HENRY, hex!("030e901c390fa37d101ff25d70594acd2df67b4493ee77a73684f25d39313536d7")),
	(IDA, hex!("03586dafcdab3d4647d4dc68732a9cab8aa34c00c5edd04e65d9dd44c2a1fd21e2")),
	(JAMES, hex!("03aec8e80ea0375f8669d6e55d7abb6a3117678d7bb851a1bd100a01e52a4fed90")),
	(KIM, hex!("03e843f200e30bc5b951c73a96d968db1c0cd05e357d910fce159fc59c40e9d6e2")),
];

pub fn account_id_to_name(account_id: &AccountId) -> &'static str {
	ALL_MOCK_VALIDATORS
		.iter()
		.find(|(_, acc_id)| acc_id == &account_id.0.0)
		.expect("Committee keys should be known")
		.0
		.name
}

impl MockValidator {
	pub const fn new(name: &'static str, seed: &'static str, stake: u64) -> Self {
		Self { name, seed, stake }
	}
	pub fn account_id(&self) -> AccountId {
		AccountId(self.ecdsa_pair().public())
	}

	pub fn ecdsa_pair(&self) -> ecdsa::Pair {
		ecdsa::Pair::from_string(self.seed, None).expect("static values are valid; qed")
	}

	pub fn sidechain_pub_key(&self) -> SidechainPublicKey {
		SidechainPublicKey(self.account_id().0.0.to_vec())
	}
	pub fn session_keys(&self) -> AccountKeys {
		AccountKeys::from_seed(self.seed)
	}

	pub fn aura_pub_key(&self) -> AuraPublicKey {
		AuraPublicKey(self.session_keys().aura.to_vec())
	}

	pub fn grandpa_pub_key(&self) -> GrandpaPublicKey {
		GrandpaPublicKey(self.session_keys().grandpa.to_vec())
	}

	pub fn keys(&self) -> CandidateKeys {
		CandidateKeys(vec![self.aura_pub_key().into(), self.grandpa_pub_key().into()])
	}
}

#[test]
fn ariadne_all_permissioned_test() {
	// P: [alice, bob]
	// R: [charlie, dave]
	// D-param: (8, 0)
	// Expected committee: [alice, bob]
	let permissioned_validators = vec![ALICE, BOB];
	let registered_validators = vec![CHARLIE, DAVE];
	let d_parameter = DParameter { num_permissioned_candidates: 8, num_registered_candidates: 0 };
	let authority_selection_inputs = create_authority_selection_inputs(
		&permissioned_validators,
		&registered_validators,
		d_parameter,
	);
	let calculated_committee = select_authorities::<AccountId, AccountKeys, TestConvertKeys>(
		UtxoId::default(),
		authority_selection_inputs,
		ScEpochNumber::zero(),
	);
	assert!(calculated_committee.is_some());

	let committee = calculated_committee.unwrap();
	let mut committee_names = committee
		.iter()
		.map(|member| account_id_to_name(member.account_id()))
		.collect::<Vec<_>>();
	committee_names.sort();
	let expected_committee_names =
		vec!["alice", "alice", "alice", "alice", "bob", "bob", "bob", "bob"];

	assert_eq!(committee_names, expected_committee_names);
}

#[test]
fn ariadne_only_permissioned_candidates_are_present_test() {
	// P: [alice, bob]
	// R: []
	// D-param: (4, 4)
	let permissioned_validators = vec![ALICE, BOB];
	let registered_validators = vec![];
	let d_parameter = DParameter { num_permissioned_candidates: 4, num_registered_candidates: 4 };
	let authority_selection_inputs = create_authority_selection_inputs(
		&permissioned_validators,
		&registered_validators,
		d_parameter,
	);
	let calculated_committee = select_authorities::<AccountId, AccountKeys, TestConvertKeys>(
		UtxoId::default(),
		authority_selection_inputs,
		ScEpochNumber::zero(),
	);
	assert!(calculated_committee.is_some());

	let committee = calculated_committee.unwrap();
	let mut committee_names = committee
		.iter()
		.map(|member| account_id_to_name(member.account_id()))
		.collect::<Vec<_>>();
	committee_names.sort();
	let expected_committee_names =
		vec!["alice", "alice", "alice", "alice", "bob", "bob", "bob", "bob"];

	assert_eq!(committee_names, expected_committee_names);
}

#[test]
fn ariadne_3_to_2_test() {
	// P: [alice, bob, charlie]
	// R: [dave, eve]
	// D-param: (3, 2)
	let permissioned_validators = vec![ALICE, BOB, CHARLIE];
	let registered_validators = vec![DAVE, EVE];
	let d_parameter = DParameter { num_permissioned_candidates: 3, num_registered_candidates: 2 };
	let authority_selection_inputs = create_authority_selection_inputs(
		&permissioned_validators,
		&registered_validators,
		d_parameter,
	);
	let calculated_committee = select_authorities::<AccountId, AccountKeys, TestConvertKeys>(
		UtxoId::default(),
		authority_selection_inputs,
		ScEpochNumber::zero(),
	);
	assert!(calculated_committee.is_some());

	let committee = calculated_committee.unwrap();
	let mut committee_names = committee
		.iter()
		.map(|member| account_id_to_name(member.account_id()))
		.collect::<Vec<_>>();
	committee_names.sort();
	let expected_committee_names = vec!["alice", "bob", "charlie", "dave", "eve"];

	assert_eq!(committee_names, expected_committee_names);
}

#[test]
fn ariadne_3_to_2_with_more_available_candidates_test() {
	// P: [alice, bob, charlie, dave, eve]
	// R: [ferdie, greg, henry, ida]
	// D-param: (3, 2)
	let permissioned_validators = vec![ALICE, BOB, CHARLIE, DAVE, EVE];
	let registered_validators = vec![FERDIE, GREG, HENRY, IDA];
	let d_parameter = DParameter { num_permissioned_candidates: 3, num_registered_candidates: 2 };
	let authority_selection_inputs = create_authority_selection_inputs(
		&permissioned_validators,
		&registered_validators,
		d_parameter,
	);
	let calculated_committee = select_authorities::<AccountId, AccountKeys, TestConvertKeys>(
		UtxoId::default(),
		authority_selection_inputs,
		ScEpochNumber::zero(),
	);
	assert!(calculated_committee.is_some());

	let committee = calculated_committee.unwrap();
	let committee_names = committee
		.iter()
		.map(|member| account_id_to_name(member.account_id()))
		.collect::<Vec<_>>();
	// No candidate has guaranteed seat. Every seat comes from random with repetitions.
	let permissioned_selected = committee_names
		.iter()
		.filter(|name| permissioned_validators.iter().any(|validator| validator.name == **name))
		.count();
	assert_eq!(permissioned_selected, 3);
	let registered_selected = committee_names
		.iter()
		.filter(|name| registered_validators.iter().any(|validator| validator.name == **name))
		.count();
	assert_eq!(registered_selected, 2);
}

#[test]
fn ariadne_4_to_7_test() {
	// P: [alice, bob, charlie, dave]
	// R: [eve, ferdie, greg, henry, ida, james, kim]
	// D-param: (4, 7)
	let permissioned_validators = vec![ALICE, BOB, CHARLIE, DAVE];
	let registered_validators = vec![EVE, FERDIE, GREG, HENRY, IDA, JAMES, KIM];
	let d_parameter = DParameter { num_permissioned_candidates: 4, num_registered_candidates: 7 };
	let authority_selection_inputs = create_authority_selection_inputs(
		&permissioned_validators,
		&registered_validators,
		d_parameter,
	);
	let calculated_committee = select_authorities::<AccountId, AccountKeys, TestConvertKeys>(
		UtxoId::default(),
		authority_selection_inputs,
		ScEpochNumber::zero(),
	);
	assert!(calculated_committee.is_some());

	let committee = calculated_committee.unwrap();
	let mut committee_names = committee
		.iter()
		.map(|member| account_id_to_name(member.account_id()))
		.collect::<Vec<_>>();
	committee_names.sort();
	// Each permissioned has 1 guaranteed. Registered candidates henry, ida, james and kim have 1 place for sure as well and three places are assigned randomly.
	let expected_committee_names = vec![
		"alice", "bob", "charlie", "dave", "eve", "eve", "ferdie", "henry", "ida", "james", "kim",
	];
	assert_eq!(committee_names, expected_committee_names);
}

#[test]
fn ariadne_does_not_return_empty_committee() {
	let authority_selection_inputs = create_authority_selection_inputs(
		&[],
		&[],
		DParameter { num_permissioned_candidates: 1, num_registered_candidates: 1 },
	);
	let calculated_committee = select_authorities::<AccountId, AccountKeys, TestConvertKeys>(
		UtxoId::default(),
		authority_selection_inputs,
		ScEpochNumber::zero(),
	);
	assert_eq!(calculated_committee, None);
}

// helpers

const DUMMY_EPOCH_NONCE: &[u8] = &[1u8, 2u8, 3u8];

fn create_epoch_candidates_idp(validators: &[MockValidator]) -> Vec<CandidateRegistrations> {
	let mainchain_key_pair: ed25519::Pair = ed25519::Pair::from_seed_slice(&[7u8; 32]).unwrap();

	let candidates: Vec<CandidateRegistrations> = validators
		.iter()
		.map(|validator| {
			let signed_message = RegisterValidatorSignedMessage {
				genesis_utxo: UtxoId::default(),
				sidechain_pub_key: validator.sidechain_pub_key().0,
				registration_utxo: UtxoId::default(),
			};

			let signed_message_encoded = minicbor::to_vec(signed_message.to_datum()).unwrap();

			let mainchain_signature = mainchain_key_pair.sign(&signed_message_encoded[..]);
			let sidechain_signature = validator.ecdsa_pair().sign(&signed_message_encoded[..]);
			let sidechain_signature_bytes_no_recovery = sidechain_signature.0[..64].to_vec();

			let registration_data = RegistrationData {
				registration_utxo: signed_message.registration_utxo,
				sidechain_signature: SidechainSignature(
					sidechain_signature_bytes_no_recovery.clone(),
				),
				mainchain_signature: MainchainSignature(mainchain_signature.0),
				cross_chain_signature: CrossChainSignature(sidechain_signature_bytes_no_recovery),
				sidechain_pub_key: validator.sidechain_pub_key(),
				keys: validator.keys(),
				cross_chain_pub_key: CrossChainPublicKey(validator.sidechain_pub_key().0),
				utxo_info: UtxoInfo::default(),
				tx_inputs: vec![signed_message.registration_utxo],
			};

			CandidateRegistrations {
				stake_pool_public_key: StakePoolPublicKey(mainchain_key_pair.public().0),
				registrations: vec![registration_data],
				stake_delegation: Some(StakeDelegation(validator.stake)),
			}
		})
		.collect();

	candidates
}

pub fn create_authority_selection_inputs(
	permissioned_candidates: &[MockValidator],
	validators: &[MockValidator],
	d_parameter: DParameter,
) -> AuthoritySelectionInputs {
	let epoch_candidates = create_epoch_candidates_idp(validators);

	let permissioned_candidates_data: Vec<PermissionedCandidateData> = permissioned_candidates
		.iter()
		.map(|c| PermissionedCandidateData {
			sidechain_public_key: c.sidechain_pub_key(),
			keys: c.keys(),
		})
		.collect();
	AuthoritySelectionInputs {
		d_parameter,
		permissioned_candidates: permissioned_candidates_data,
		registered_candidates: epoch_candidates,
		epoch_nonce: EpochNonce(DUMMY_EPOCH_NONCE.to_vec()),
	}
}

mod maybe_from_candidate_keys_tests {
	use crate::{ConvertForImplOpaqueKeys, MaybeFromCandidateKeys};
	use sidechain_domain::{CandidateKey, CandidateKeys};
	use sp_runtime::{BoundToRuntimeAppPublic, RuntimeAppPublic, impl_opaque_keys};

	pub struct KeyModule;
	impl BoundToRuntimeAppPublic for KeyModule {
		type Public = sp_runtime::app_crypto::ed25519::AppPublic;
	}

	pub struct KeyModule2;
	impl BoundToRuntimeAppPublic for KeyModule2 {
		type Public = sp_runtime::app_crypto::sr25519::AppPublic;
	}

	impl_opaque_keys! {
		pub struct Keys {
			pub k1: KeyModule,
			pub k2: KeyModule2,
		}
	}

	#[test]
	fn convert_for_impl_opaque_keys_extracts_keys_by_opaque_keys_order() {
		let keys: Keys = ConvertForImplOpaqueKeys::maybe_from(&CandidateKeys(vec![
			CandidateKey {
				id: <KeyModule as BoundToRuntimeAppPublic>::Public::ID.0,
				bytes: [1u8; 32].to_vec(),
			},
			CandidateKey {
				id: <KeyModule2 as BoundToRuntimeAppPublic>::Public::ID.0,
				bytes: [2u8; 32].to_vec(),
			},
		]))
		.unwrap();
		assert_eq!(
			keys,
			Keys {
				k1: sp_core::ed25519::Public::from([1u8; 32]).into(),
				k2: sp_core::sr25519::Public::from([2u8; 32]).into(),
			}
		)
	}
}
