use crate::{PlutusDataExtensions, VersionedDatum, VersionedGenericDatumShape};
use cardano_serialization_lib::*;
use sidechain_domain::*;

use crate::{DataDecodingError, DecodingResult};

/** Representation of the plutus type in the mainchain contract (rev 4ed2cc66c554ec8c5bec7b90ad9273e9069a1fb4)
 *
 * Note that the ECDSA secp256k1 public key is serialized in compressed format and the
 * sidechain signature does not contain the recovery bytes (it's just r an s concatenated).
 *
 * data BlockProducerRegistration = BlockProducerRegistration
 * { -- | Verification keys required by the stake ownership model
 *   -- | @since v4.0.0
 *  stakeOwnership :: StakeOwnership
 * , -- | public key in the sidechain's desired format
 *  sidechainPubKey :: LedgerBytes
 * , -- | Signature of the sidechain
 *   -- | @since v4.0.0
 *  sidechainSignature :: Signature
 * , -- | A UTxO that must be spent by the transaction
 *   -- | @since v4.0.0
 *  inputUtxo :: TxOutRef
 * , -- | Owner public key hash
 *   -- | @since v4.0.0
 *  ownPkh :: PubKeyHash
 * , -- | Sidechain authority discovery key
 *   -- | @since Unreleased
 *   auraKey :: LedgerBytes
 * , -- | Sidechain grandpa key
 *   -- | @since Unreleased
 *   grandpaKey :: LedgerBytes
 * }
 */
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegisterValidatorDatum {
	/// Initial/legacy datum schema. If a datum doesn't contain a version, it is assumed to be V0
	V0 {
		stake_ownership: AdaBasedStaking,
		sidechain_pub_key: SidechainPublicKey,
		sidechain_signature: SidechainSignature,
		consumed_input: UtxoId,
		//own_pkh is used by offchain code to find the registration UTXO when re-registering or deregistering
		own_pkh: MainchainAddressHash,
		aura_pub_key: AuraPublicKey,
		grandpa_pub_key: GrandpaPublicKey,
	},
}

/// AdaBasedStaking is a variant of Plutus type StakeOwnership.
/// The other variant, TokenBasedStaking, is not supported
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdaBasedStaking {
	pub pub_key: MainchainPublicKey,
	pub signature: MainchainSignature,
}

impl TryFrom<PlutusData> for RegisterValidatorDatum {
	type Error = DataDecodingError;

	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		Self::decode(&datum)
	}
}

impl VersionedDatum for RegisterValidatorDatum {
	const NAME: &str = "RegisterValidatorDatum";

	fn decode_legacy(data: &PlutusData) -> Result<Self, String> {
		decode_legacy_register_validator_datum(data).ok_or_else(|| "Invalid data".into())
	}

	fn decode_versioned(
		version: u32,
		const_data: &PlutusData,
		mut_data: &PlutusData,
	) -> Result<Self, String> {
		match version {
			0 => decode_v0_register_validator_datum(const_data, mut_data)
				.ok_or("Can not parse mutable part of data".to_string()),
			_ => Err(format!("Unknown version: {version}")),
		}
	}
}

fn decode_v0_register_validator_datum(
	const_data: &PlutusData,
	mut_data: &PlutusData,
) -> Option<RegisterValidatorDatum> {
	let fields = mut_data
		.as_constr_plutus_data()
		.filter(|datum| datum.alternative().is_zero())
		.filter(|datum| datum.data().len() >= 6)?
		.data();
	let stake_ownership = decode_ada_based_staking_datum(fields.get(0))?;
	let sidechain_pub_key = fields.get(1).as_bytes().map(SidechainPublicKey)?;
	let sidechain_signature = fields.get(2).as_bytes().map(SidechainSignature)?;
	let consumed_input = decode_utxo_id_datum(fields.get(3))?;
	let aura_pub_key = fields.get(4).as_bytes().map(AuraPublicKey)?;
	let grandpa_pub_key = fields.get(5).as_bytes().map(GrandpaPublicKey)?;

	let own_pkh = MainchainAddressHash(const_data.as_bytes()?.try_into().ok()?);
	Some(RegisterValidatorDatum::V0 {
		stake_ownership,
		sidechain_pub_key,
		sidechain_signature,
		consumed_input,
		own_pkh,
		aura_pub_key,
		grandpa_pub_key,
	})
}

/// Parses plutus data schema that was used before datum versioning was added. Kept for backwards compatibility.
fn decode_legacy_register_validator_datum(datum: &PlutusData) -> Option<RegisterValidatorDatum> {
	let fields = datum
		.as_constr_plutus_data()
		.filter(|datum| datum.alternative().is_zero())
		.filter(|datum| datum.data().len() >= 7)?
		.data();
	let stake_ownership = decode_ada_based_staking_datum(fields.get(0))?;
	let sidechain_pub_key = fields.get(1).as_bytes().map(SidechainPublicKey)?;
	let sidechain_signature = fields.get(2).as_bytes().map(SidechainSignature)?;
	let consumed_input = decode_utxo_id_datum(fields.get(3))?;
	let own_pkh = MainchainAddressHash(fields.get(4).as_bytes()?.try_into().ok()?);
	let aura_pub_key = fields.get(5).as_bytes().map(AuraPublicKey)?;
	let grandpa_pub_key = fields.get(6).as_bytes().map(GrandpaPublicKey)?;
	Some(RegisterValidatorDatum::V0 {
		stake_ownership,
		sidechain_pub_key,
		sidechain_signature,
		consumed_input,
		own_pkh,
		aura_pub_key,
		grandpa_pub_key,
	})
}

fn decode_ada_based_staking_datum(datum: PlutusData) -> Option<AdaBasedStaking> {
	let fields = datum
		.as_constr_plutus_data()
		.filter(|datum| datum.alternative().is_zero())
		.filter(|datum| datum.data().len() >= 2)?
		.data();
	let pub_key = TryFrom::try_from(fields.get(0).as_bytes()?).ok()?;
	let signature = MainchainSignature(fields.get(1).as_bytes()?);
	Some(AdaBasedStaking { pub_key, signature })
}
fn decode_utxo_id_datum(datum: PlutusData) -> Option<UtxoId> {
	let fields = datum
		.as_constr_plutus_data()
		.filter(|datum| datum.alternative().is_zero())
		.filter(|datum| datum.data().len() >= 2)?
		.data();
	let tx_hash = decode_tx_hash_datum(fields.get(0))?;
	let index = UtxoIndex(fields.get(1).as_u16()?);
	Some(UtxoId { tx_hash, index })
}
/// Plutus type for TxHash is a sum type, we can parse only variant with constructor 0.
fn decode_tx_hash_datum(datum: PlutusData) -> Option<McTxHash> {
	let constructor_datum = datum
		.as_constr_plutus_data()
		.filter(|datum| datum.alternative().is_zero())
		.filter(|datum| datum.data().len() >= 1)?;
	let bytes = constructor_datum.data().get(0).as_bytes()?;

	Some(McTxHash(TryFrom::try_from(bytes).ok()?))
}

impl From<RegisterValidatorDatum> for PlutusData {
	fn from(value: RegisterValidatorDatum) -> Self {
		match value {
			RegisterValidatorDatum::V0 {
				stake_ownership,
				sidechain_pub_key,
				sidechain_signature,
				consumed_input,
				own_pkh,
				aura_pub_key,
				grandpa_pub_key,
			} => {
				let mut generic_data_fields = PlutusList::new();
				generic_data_fields.add(&stake_ownership_to_plutus_data(stake_ownership));
				generic_data_fields.add(&PlutusData::new_bytes(sidechain_pub_key.0));
				generic_data_fields.add(&PlutusData::new_bytes(sidechain_signature.0));
				generic_data_fields.add(&utxo_id_to_plutus_data(consumed_input));
				generic_data_fields.add(&PlutusData::new_bytes(aura_pub_key.0));
				generic_data_fields.add(&PlutusData::new_bytes(grandpa_pub_key.0));
				let generic_data = ConstrPlutusData::new(&BigNum::zero(), &generic_data_fields);
				VersionedGenericDatumShape {
					datum: PlutusData::new_bytes(own_pkh.0.to_vec()),
					generic_data: PlutusData::new_constr_plutus_data(&generic_data),
					version: 0,
				}
				.into()
			},
		}
	}
}

fn stake_ownership_to_plutus_data(v: AdaBasedStaking) -> PlutusData {
	let mut fields = PlutusList::new();
	fields.add(&PlutusData::new_bytes(v.pub_key.0.to_vec()));
	fields.add(&PlutusData::new_bytes(v.signature.0.to_vec()));
	PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(&BigNum::zero(), &fields))
}

fn utxo_id_to_plutus_data(v: UtxoId) -> PlutusData {
	let mut fields = PlutusList::new();
	fields.add(&tx_hash_to_plutus_data(v.tx_hash));
	fields.add(&PlutusData::new_integer(&v.index.0.into()));
	PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(&BigNum::zero(), &fields))
}

fn tx_hash_to_plutus_data(v: McTxHash) -> PlutusData {
	let mut fields = PlutusList::new();
	fields.add(&PlutusData::new_bytes(v.0.to_vec()));
	PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(&BigNum::zero(), &fields))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_helpers::*;
	use hex_literal::hex;
	use pretty_assertions::assert_eq;

	fn test_datum_v0() -> RegisterValidatorDatum {
		RegisterValidatorDatum::V0 {
			stake_ownership: AdaBasedStaking {
				pub_key: MainchainPublicKey(hex!("bfbee74ab533f40979101057f96de62e95233f2a5216eb16b54106f09fd7350d")),
				signature: MainchainSignature(hex!("28d1c3b7df297a60d24a3f88bc53d7029a8af35e8dd876764fd9e7a24203a3482a98263cc8ba2ddc7dc8e7faea31c2e7bad1f00e28c43bc863503e3172dc6b0a").into()),
			},
			sidechain_pub_key: SidechainPublicKey(hex!("02fe8d1eb1bcb3432b1db5833ff5f2226d9cb5e65cee430558c18ed3a3c86ce1af").into()),
			sidechain_signature: SidechainSignature(hex!("f8ec6c7f935d387aaa1693b3bf338cbb8f53013da8a5a234f9c488bacac01af259297e69aee0df27f553c0a1164df827d016125c16af93c99be2c19f36d2f66e").into()),
			consumed_input: UtxoId {
				tx_hash: McTxHash(hex!("cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13")),
				index: UtxoIndex(1),
			},
			own_pkh: MainchainAddressHash(hex!("aabbccddeeff00aabbccddeeff00aabbccddeeff00aabbccddeeff00")),
			aura_pub_key: AuraPublicKey(hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d").into()),
			grandpa_pub_key: GrandpaPublicKey(hex!("88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee").into()),
		}
	}

	#[test]
	fn valid_legacy_registration() {
		let plutus_data = test_plutus_data!({
			"constructor": 0,
			"fields": [
				{
					"constructor": 0,
					"fields": [
						{ "bytes": "bfbee74ab533f40979101057f96de62e95233f2a5216eb16b54106f09fd7350d" },
						{ "bytes": "28d1c3b7df297a60d24a3f88bc53d7029a8af35e8dd876764fd9e7a24203a3482a98263cc8ba2ddc7dc8e7faea31c2e7bad1f00e28c43bc863503e3172dc6b0a" }
					]
				},
				{ "bytes": "02fe8d1eb1bcb3432b1db5833ff5f2226d9cb5e65cee430558c18ed3a3c86ce1af" },
				{ "bytes": "f8ec6c7f935d387aaa1693b3bf338cbb8f53013da8a5a234f9c488bacac01af259297e69aee0df27f553c0a1164df827d016125c16af93c99be2c19f36d2f66e" },
				{
					"fields": [
						{
							"constructor": 0,
							"fields": [ { "bytes": "cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"} ]
						},
						{ "int": 1 }
					],
					"constructor": 0
				},
				{ "bytes": "aabbccddeeff00aabbccddeeff00aabbccddeeff00aabbccddeeff00" },
				{ "bytes": "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d" },
				{ "bytes": "88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee" }
			]
		});
		assert_eq!(RegisterValidatorDatum::try_from(plutus_data).unwrap(), test_datum_v0())
	}

	fn test_versioned_datum_json() -> serde_json::Value {
		serde_json::json!({
			"list": [
				{ "bytes": "aabbccddeeff00aabbccddeeff00aabbccddeeff00aabbccddeeff00" },
				{
					"constructor": 0,
					"fields": [
						{
							"constructor": 0,
							"fields": [
								{ "bytes": "bfbee74ab533f40979101057f96de62e95233f2a5216eb16b54106f09fd7350d" },
								{ "bytes": "28d1c3b7df297a60d24a3f88bc53d7029a8af35e8dd876764fd9e7a24203a3482a98263cc8ba2ddc7dc8e7faea31c2e7bad1f00e28c43bc863503e3172dc6b0a" }
							]
						},
						{ "bytes": "02fe8d1eb1bcb3432b1db5833ff5f2226d9cb5e65cee430558c18ed3a3c86ce1af" },
						{ "bytes": "f8ec6c7f935d387aaa1693b3bf338cbb8f53013da8a5a234f9c488bacac01af259297e69aee0df27f553c0a1164df827d016125c16af93c99be2c19f36d2f66e" },
						{
							"fields": [
								{
									"constructor": 0,
									"fields": [ { "bytes": "cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"} ]
								},
								{ "int": 1 }
							],
							"constructor": 0
						},
						{ "bytes": "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d" },
						{ "bytes": "88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee" }
					]
				},
				{ "int": 0 }
			]
		})
	}

	#[test]
	fn valid_v0_registration_from_plutus_data() {
		let plutus_data = json_to_plutus_data(test_versioned_datum_json());
		assert_eq!(RegisterValidatorDatum::try_from(plutus_data).unwrap(), test_datum_v0())
	}

	#[test]
	fn v0_registration_to_plutus_data() {
		let plutus_data: PlutusData = test_datum_v0().into();
		assert_eq!(plutus_data_to_json(plutus_data), test_versioned_datum_json())
	}
}
