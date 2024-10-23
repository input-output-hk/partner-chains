use crate::candidates::{
	AuraPublicKey, GrandpaPublicKey, MainchainSignature, McTxHash, SidechainPublicKey,
	SidechainSignature, UtxoId, UtxoIndex,
};
use crate::Datum::{self, ByteStringDatum, ConstructorDatum, IntegerDatum};
use sidechain_domain::*;

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
#[derive(Clone, Debug)]
pub enum RegisterValidatorDatum {
	/// Initial/legacy datum schema. If a datum doesn't contain a version, it is assumed to be V0
	V0 {
		stake_ownership: AdaBasedStaking,
		sidechain_pub_key: SidechainPublicKey,
		sidechain_signature: SidechainSignature,
		consumed_input: UtxoId,
		//ownPkh we don't use,
		aura_pub_key: AuraPublicKey,
		grandpa_pub_key: GrandpaPublicKey,
	},
}

/// AdaBasedStaking is a variant of Plutus type StakeOwnership.
/// The other variant, TokenBasedStaking, is not supported
#[derive(Clone, Debug)]
pub struct AdaBasedStaking {
	pub pub_key: MainchainPublicKey,
	pub signature: MainchainSignature,
}

impl TryFrom<&Datum> for RegisterValidatorDatum {
	type Error = super::Error;

	fn try_from(datum: &Datum) -> super::Result<Self> {
		decode_legacy_register_validator_datum(datum).ok_or("Invalid registration datum".into())
	}
}

pub fn decode_legacy_register_validator_datum(datum: &Datum) -> Option<RegisterValidatorDatum> {
	match datum {
		ConstructorDatum { constructor: 0, fields } => {
			let stake_ownership = fields.first().and_then(decode_ada_based_staking_datum)?;
			let sidechain_pub_key = fields
				.get(1)
				.and_then(|d| d.as_bytestring())
				.map(|bytes| SidechainPublicKey(bytes.clone()))?;
			let sidechain_signature = fields
				.get(2)
				.and_then(|d| d.as_bytestring())
				.map(|bytes| SidechainSignature(bytes.clone()))?;
			let consumed_input = fields.get(3).and_then(decode_utxo_id_datum)?;
			let _own_pkh = fields.get(4).and_then(|d| d.as_bytestring())?;
			let aura_pub_key = fields
				.get(5)
				.and_then(|d| d.as_bytestring())
				.map(|bytes| AuraPublicKey(bytes.clone()))?;
			let grandpa_pub_key = fields
				.get(6)
				.and_then(|d| d.as_bytestring())
				.map(|bytes| GrandpaPublicKey(bytes.clone()))?;
			Some(RegisterValidatorDatum::V0 {
				stake_ownership,
				sidechain_pub_key,
				sidechain_signature,
				consumed_input,
				aura_pub_key,
				grandpa_pub_key,
			})
		},

		_ => None,
	}
}

fn decode_ada_based_staking_datum(datum: &Datum) -> Option<AdaBasedStaking> {
	match datum {
		ConstructorDatum { constructor: 0, fields } => match fields.first().zip(fields.get(1)) {
			Some((ByteStringDatum(f0), ByteStringDatum(f1))) => {
				let pub_key = TryFrom::try_from(f0.clone()).ok()?;
				Some(AdaBasedStaking { pub_key, signature: MainchainSignature(f1.clone()) })
			},
			_ => None,
		},
		_ => None,
	}
}
fn decode_utxo_id_datum(datum: &Datum) -> Option<UtxoId> {
	match datum {
		ConstructorDatum { constructor: 0, fields } => match fields.first().zip(fields.get(1)) {
			Some((f0, IntegerDatum(f1))) => {
				let tx_hash = decode_tx_hash_datum(f0)?;
				let index: u16 = TryFrom::try_from(f1.clone()).ok()?;
				Some(UtxoId { tx_hash, index: UtxoIndex(index) })
			},
			_ => None,
		},
		_ => None,
	}
}
/// Plutus type for TxHash is a sum type, we can parse only variant with constructor 0.
fn decode_tx_hash_datum(datum: &Datum) -> Option<McTxHash> {
	match datum {
		ConstructorDatum { constructor: 0, fields } => {
			let bytes = fields.first().and_then(|d| d.as_bytestring())?;
			Some(McTxHash(TryFrom::try_from(bytes.clone()).ok()?))
		},
		_ => None,
	}
}
