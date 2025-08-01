use crate::key_params::{SidechainSigningKeyParam, StakePoolSigningKeyParam};
use clap::Parser;
use plutus_datum_derive::*;
use secp256k1::SecretKey;
use serde::Serialize;
use serde_json;
use sidechain_domain::{
	MainchainSignature, SidechainPublicKey, SidechainSignature, StakePoolPublicKey, UtxoId,
	crypto::*,
};
use std::fmt::{Display, Formatter};

/// Generates dual signatures (Ed25519 + ECDSA) for Partner Chain validator registration.
#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct RegistrationSignaturesCmd {
	/// Genesis UTXO that uniquely identifies the target Partner Chain
	#[arg(long)]
	pub genesis_utxo: UtxoId,
	/// Bytes of the Cardano Stake Pool Signing Key. Bytes of 'cbor' field of a Cardano key file content, after dropping the '5820' prefix.
	#[arg(long)]
	pub mainchain_signing_key: StakePoolSigningKeyParam,
	/// ECDSA private key for the Partner Chain validator
	#[arg(long)]
	pub sidechain_signing_key: SidechainSigningKeyParam,
	/// UTXO to be spend during validator registration transaction
	#[arg(long)]
	pub registration_utxo: UtxoId,
}

impl RegistrationSignaturesCmd {
	/// Creates the structured message that will be signed by both mainchain and sidechain keys.
	pub fn to_register_validator_message(&self, genesis_utxo: UtxoId) -> RegisterValidatorMessage {
		RegisterValidatorMessage::new(
			genesis_utxo,
			self.sidechain_signing_key.to_pub_key(),
			self.registration_utxo,
		)
	}

	/// Generates mainchain and sidechain signatures with public keys.
	pub fn execute(&self) -> RegistrationCmdOutput {
		self.to_register_validator_message(self.genesis_utxo)
			.sign_and_prepare_registration_cmd_output(
				self.mainchain_signing_key.0,
				self.sidechain_signing_key.0,
			)
	}
}

/// Complete registration output with signatures and public keys for both chains.
#[derive(Clone, Debug, Serialize)]
pub struct RegistrationCmdOutput {
	/// Ed25519 public key of the Cardano stake pool operator
	pub spo_public_key: StakePoolPublicKey,
	/// Ed25519 signature from the stake pool operator
	pub spo_signature: MainchainSignature,
	/// ECDSA public key for Partner Chain operations
	pub sidechain_public_key: SidechainPublicKey,
	/// ECDSA signature from the Partner Chain validator key
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

/// Message structure for validator registration signatures.
#[derive(Clone, Debug, ToDatum)]
pub struct RegisterValidatorMessage {
	/// Genesis UTXO identifying the specific Partner Chain instance
	pub genesis_utxo: UtxoId,
	/// ECDSA public key for the validator on the Partner Chain
	pub sidechain_pub_key: SidechainPublicKey,
	/// UTXO consumed in the registration transaction for uniqueness
	pub registration_utxo: UtxoId,
}

impl RegisterValidatorMessage {
	/// Creates new validator registration message.
	pub fn new(
		genesis_utxo: UtxoId,
		pub_key: secp256k1::PublicKey,
		registration_utxo: UtxoId,
	) -> Self {
		RegisterValidatorMessage {
			genesis_utxo,
			sidechain_pub_key: SidechainPublicKey(pub_key.serialize().to_vec()),
			registration_utxo,
		}
	}

	/// Signs message with both mainchain and sidechain keys.
	pub fn sign_and_prepare_registration_cmd_output(
		&self,
		mainchain_key: ed25519_zebra::SigningKey,
		sidechain_key: SecretKey,
	) -> RegistrationCmdOutput {
		let (spo_public_key, spo_signature) =
			cardano_spo_public_key_and_signature(mainchain_key, self.clone());
		let (sc_pub_key, sc_signature) =
			sc_public_key_and_signature_for_datum(sidechain_key, self.clone());
		RegistrationCmdOutput {
			spo_public_key,
			spo_signature,
			sidechain_public_key: SidechainPublicKey(sc_pub_key.serialize().to_vec()),
			sidechain_signature: SidechainSignature(sc_signature.serialize_compact().to_vec()),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::registration_signatures::RegisterValidatorMessage;
	use plutus::to_datum_cbor_bytes;
	use secp256k1::PublicKey;
	use sidechain_domain::UtxoId;
	use std::str::FromStr;

	#[test]
	fn validator_msg_to_datum() {
		let sidechain_pub_key = PublicKey::from_str(
			"02dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ec",
		)
		.unwrap();
		let genesis_utxo =
			UtxoId::from_str("e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efa#4")
				.unwrap();
		let registration_utxo =
			UtxoId::from_str("8ea10040249ad3033ae7c4d4b69e0b2e2b50a90741b783491cb5ddf8ced0d861#4")
				.unwrap();
		let message =
			RegisterValidatorMessage::new(genesis_utxo, sidechain_pub_key, registration_utxo);
		assert_eq!(
			hex::encode(to_datum_cbor_bytes(message)),
			"d8799fd8799fd8799f5820e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efaff04ff582102dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ecd8799fd8799f58208ea10040249ad3033ae7c4d4b69e0b2e2b50a90741b783491cb5ddf8ced0d861ff04ffff"
		)
	}
}
