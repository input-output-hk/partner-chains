use crate::key_params::{MainchainSigningKeyParam, SidechainSigningKeyParam};
use crate::signing::{mainchain_public_key_and_signature, sc_public_key_and_signature_for_datum};
use clap::{Args, Parser};
use plutus::{Datum, ToDatum};
use plutus_datum_derive::*;
use secp256k1::SecretKey;
use serde::Serialize;
use serde_json;
use sidechain_domain::{
	MainchainPublicKey, MainchainSignature, SidechainPublicKey, SidechainSignature, UtxoId,
};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct RegistrationSignaturesCmd<SP: Args> {
	#[clap(flatten)]
	pub sidechain_params: SP,
	#[arg(long)]
	pub mainchain_signing_key: MainchainSigningKeyParam,
	#[arg(long)]
	pub sidechain_signing_key: SidechainSigningKeyParam,
	#[arg(long)]
	pub registration_utxo: UtxoId,
}

impl<SP: Args + ToDatum + Clone> RegistrationSignaturesCmd<SP> {
	pub fn to_register_validator_message(
		&self,
		sidechain_params: SP,
	) -> RegisterValidatorMessage<SP> {
		RegisterValidatorMessage::new(
			sidechain_params,
			self.sidechain_signing_key.to_pub_key(),
			self.registration_utxo,
		)
	}

	pub fn execute(&self) -> RegistrationCmdOutput {
		self.to_register_validator_message(self.sidechain_params.clone())
			.sign_and_prepare_registration_cmd_output(
				self.mainchain_signing_key.0,
				self.sidechain_signing_key.0,
			)
	}
}

#[derive(Clone, Debug, Serialize)]
pub struct RegistrationCmdOutput {
	pub spo_public_key: MainchainPublicKey,
	pub spo_signature: MainchainSignature,
	pub sidechain_public_key: SidechainPublicKey,
	pub sidechain_signature: SidechainSignature,
}

impl Display for RegistrationCmdOutput {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match serde_json::to_string(self) {
			Ok(json) => write!(f, "{}", json),
			Err(e) => write!(f, "{{'error': '{}'}}", e),
		}
	}
}

#[derive(Clone, Debug, ToDatum)]
pub struct RegisterValidatorMessage<SidechainParams> {
	pub sidechain_params: SidechainParams,
	pub sidechain_pub_key: SidechainPublicKey,
	pub input_utxo: UtxoId,
}

impl<SidechainParams: ToDatum + Clone> RegisterValidatorMessage<SidechainParams> {
	pub fn new(
		sidechain_params: SidechainParams,
		pub_key: secp256k1::PublicKey,
		input_utxo: UtxoId,
	) -> Self {
		RegisterValidatorMessage {
			sidechain_params,
			sidechain_pub_key: SidechainPublicKey(pub_key.serialize().to_vec()),
			input_utxo,
		}
	}

	pub fn sign_and_prepare_registration_cmd_output(
		&self,
		mainchain_key: ed25519_zebra::SigningKey,
		sidechain_key: SecretKey,
	) -> RegistrationCmdOutput {
		let (mc_pub_key, mc_signature) =
			mainchain_public_key_and_signature(mainchain_key, self.clone());
		let (sc_pub_key, sc_signature) =
			sc_public_key_and_signature_for_datum(sidechain_key, self.clone());
		RegistrationCmdOutput {
			spo_public_key: MainchainPublicKey(mc_pub_key.into()),
			spo_signature: MainchainSignature(mc_signature.to_vec()),
			sidechain_public_key: SidechainPublicKey(sc_pub_key.serialize().to_vec()),
			sidechain_signature: SidechainSignature(sc_signature.serialize_compact().to_vec()),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::registration_signatures::RegisterValidatorMessage;
	use chain_params::SidechainParams;
	use plutus::to_datum_cbor_bytes;
	use secp256k1::PublicKey;
	use sidechain_domain::{MainchainAddressHash, UtxoId};
	use std::str::FromStr;

	#[test]
	fn validator_msg_to_datum() {
		let sidechain_pub_key = PublicKey::from_str(
			"02dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ec",
		)
		.unwrap();
		let sidechain_params = SidechainParams {
			genesis_committee_utxo: UtxoId::from_str(
				"e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efa#4",
			)
			.unwrap(),
			governance_authority: MainchainAddressHash::from_hex_unsafe(
				"4f2d6145e1700ad11dc074cad9f4194cc53b0dbab6bd25dfea6c501a",
			),
		};
		let input_utxo = sidechain_params.genesis_committee_utxo;
		let message =
			RegisterValidatorMessage::new(sidechain_params, sidechain_pub_key, input_utxo);
		assert_eq!(hex::encode(to_datum_cbor_bytes(message)), "d8799fd8799f0bd8799fd8799f5820e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efaff04ff0203581c4f2d6145e1700ad11dc074cad9f4194cc53b0dbab6bd25dfea6c501aff582102dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ecd8799fd8799f5820e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efaff04ffff")
	}
}
