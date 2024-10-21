use db_sync_follower::candidates::{AdaBasedStaking, RegisterValidatorDatum};
use pallas_primitives::alonzo::{BigInt, PlutusData};
use sidechain_domain::{
	AuraPublicKey, GrandpaPublicKey, McTxHash, SidechainPublicKey, SidechainSignature, UtxoId,
};

pub fn decode_register_validator_datum(datum: &PlutusData) -> Option<RegisterValidatorDatum> {
	match datum {
		PlutusData::Constr(constr) => {
			let fields = &constr.fields;
			let stake_ownership = fields.first().and_then(decode_ada_based_staking_datum)?;
			let sidechain_pub_key = fields
				.get(1)
				.and_then(|d| decode_bytestring(d))
				.map(|bytes| SidechainPublicKey(bytes.clone()))?;
			let sidechain_signature = fields
				.get(2)
				.and_then(|d| decode_bytestring(d))
				.map(|bytes| SidechainSignature(bytes.clone()))?;
			let consumed_input = fields.get(3).and_then(decode_utxo_id_datum)?;
			let own_pkh = fields
				.get(4)
				.and_then(|d| decode_bytestring(d))
				.and_then(|bytes| TryFrom::try_from(bytes.clone()).ok())?;
			let aura_pub_key = fields
				.get(5)
				.and_then(|d| decode_bytestring(d))
				.map(|bytes| AuraPublicKey(bytes.clone()))?;
			let grandpa_pub_key = fields
				.get(6)
				.and_then(|d| decode_bytestring(d))
				.map(|bytes| GrandpaPublicKey(bytes.clone()))?;
			Some(RegisterValidatorDatum {
				stake_ownership,
				sidechain_pub_key,
				sidechain_signature,
				consumed_input,
				own_pkh,
				aura_pub_key,
				grandpa_pub_key,
			})
		},

		_ => None,
	}
}

fn decode_ada_based_staking_datum(datum: &PlutusData) -> Option<AdaBasedStaking> {
	match datum {
		PlutusData::Constr(constr) => match constr.fields.first().zip(constr.fields.get(1)) {
			Some((PlutusData::BoundedBytes(f0), PlutusData::BoundedBytes(f1))) => {
				let f0: Vec<u8> = f0.clone().into();
				let pub_key = TryFrom::try_from(f0).ok()?;
				let f1: Vec<u8> = f1.clone().into();
				Some(AdaBasedStaking {
					pub_key,
					signature: sidechain_domain::MainchainSignature(f1),
				})
			},
			_ => None,
		},
		_ => None,
	}
}

fn decode_utxo_id_datum(datum: &PlutusData) -> Option<UtxoId> {
	match datum {
		PlutusData::Constr(constr) => match constr.fields.first().zip(constr.fields.get(1)) {
			Some((f0, f1)) => {
				let tx_hash = decode_tx_hash_datum(f0)?;
				let index: u16 = decode_u16(f1)?;
				Some(UtxoId { tx_hash, index: sidechain_domain::UtxoIndex(index) })
			},
			_ => None,
		},
		_ => None,
	}
}

pub fn decode_u16(pd: &PlutusData) -> Option<u16> {
	match pd {
		PlutusData::BigInt(BigInt::Int(i)) => TryFrom::try_from(**i).ok(),
		_ => None,
	}
}

fn decode_tx_hash_datum(datum: &PlutusData) -> Option<McTxHash> {
	match datum {
		PlutusData::Constr(constr) => {
			let bytes = constr.fields.first().and_then(|d| decode_bytestring(d))?;
			Some(McTxHash(TryFrom::try_from(bytes.clone()).ok()?))
		},
		_ => None,
	}
}

fn decode_bytestring(d: &PlutusData) -> Option<&Vec<u8>> {
	match d {
		PlutusData::BoundedBytes(bb) => Some(bb),
		_ => None,
	}
}
