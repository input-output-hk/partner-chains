use crate::PlutusDataExtensions;
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
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub struct AdaBasedStaking {
	pub pub_key: MainchainPublicKey,
	pub signature: MainchainSignature,
}

impl TryFrom<PlutusData> for RegisterValidatorDatum {
	type Error = DataDecodingError;

	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		decode_legacy_register_validator_datum(&datum).ok_or(DataDecodingError {
			datum,
			to: "RegisterValidatorDatum".to_string(),
			msg: "Invalid Plutus data".to_string(),
		})
	}
}

/// Parses plutus data schema that was used before datum versioning was added. Kept for backwards compatibility.
pub fn decode_legacy_register_validator_datum(
	datum: &PlutusData,
) -> Option<RegisterValidatorDatum> {
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
