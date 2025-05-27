//! # Validator Registration Signatures
//!
//! Generate dual signatures for validator registration in Partner Chains.
//! This module creates both mainchain (Cardano) and sidechain signatures
//! required for validator registration and participation.
//!
//! ## Process Overview
//!
//! 1. Create registration message with genesis UTXO, sidechain public key, and registration UTXO
//! 2. Generate mainchain signature using Cardano stake pool signing key (Ed25519)
//! 3. Generate sidechain signature using Partner Chain signing key (secp256k1)
//! 4. Output both signatures with corresponding public keys in JSON format
//!
//! ## Dual Signature System
//!
//! Validator registration requires signatures from both:
//! - **Mainchain (Cardano)**: Proves ownership of Cardano stake pool
//! - **Sidechain**: Proves ownership of Partner Chain validator key
//!
//! ## CLI Integration
//!
//! ```bash
//! partner-chains-cli registration-signatures \
//!   --genesis-utxo e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efa#4 \
//!   --mainchain-signing-key 2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c \
//!   --sidechain-signing-key 02dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ec \
//!   --registration-utxo 8ea10040249ad3033ae7c4d4b69e0b2e2b50a90741b783491cb5ddf8ced0d861#4
//! ```
//!
//! ## Output Format
//!
//! ```json
//! {
//!   "spo_public_key": "2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c",
//!   "spo_signature": "...",
//!   "sidechain_public_key": "02dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ec",
//!   "sidechain_signature": "..."
//! }
//! ```
//!
//! ## CBOR Datum Generation
//!
//! The registration message is encoded as Plutus CBOR datum for on-chain verification.
//! This ensures compatibility with Cardano smart contracts and validation logic.

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

/// Command for generating validator registration signatures.
///
/// Creates both mainchain and sidechain signatures required for validator registration.
/// The dual signature system proves ownership of both Cardano stake pool keys and
/// Partner Chain validator keys.
///
/// ## Parameters
///
/// - `genesis_utxo`: Identifies the target Partner Chain
/// - `mainchain_signing_key`: Cardano stake pool signing key (Ed25519)
/// - `sidechain_signing_key`: Partner Chain validator signing key (secp256k1)
/// - `registration_utxo`: UTXO used for registration transaction
///
/// ## Example Usage
///
/// ```bash
/// partner-chains-cli registration-signatures \
///   --genesis-utxo e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efa#4 \
///   --mainchain-signing-key 2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c \
///   --sidechain-signing-key 02dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ec \
///   --registration-utxo 8ea10040249ad3033ae7c4d4b69e0b2e2b50a90741b783491cb5ddf8ced0d861#4
/// ```
///
/// ## Security Requirements
///
/// Both signing keys must be kept secure as they represent:
/// - Mainchain key: Authority over Cardano stake pool operations
/// - Sidechain key: Authority over Partner Chain validator operations
#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct RegistrationSignaturesCmd {
	/// Genesis UTXO of the target Partner Chain
	#[arg(long)]
	pub genesis_utxo: UtxoId,
	/// Bytes of the Cardano Stake Pool Signing Key. Bytes of 'cbor' field of a Cardano key file content, after dropping the '5820' prefix.
	#[arg(long)]
	pub mainchain_signing_key: StakePoolSigningKeyParam,
	/// Partner Chain validator signing key for sidechain operations
	#[arg(long)]
	pub sidechain_signing_key: SidechainSigningKeyParam,
	/// UTXO used for validator registration transaction
	#[arg(long)]
	pub registration_utxo: UtxoId,
}

impl RegistrationSignaturesCmd {
	/// Create registration message from command parameters.
	///
	/// Constructs a `RegisterValidatorMessage` containing the genesis UTXO,
	/// sidechain public key derived from the signing key, and registration UTXO.
	///
	/// ## Parameters
	///
	/// - `genesis_utxo`: Genesis UTXO identifying the Partner Chain
	///
	/// ## Returns
	///
	/// `RegisterValidatorMessage` ready for signature generation.
	pub fn to_register_validator_message(&self, genesis_utxo: UtxoId) -> RegisterValidatorMessage {
		RegisterValidatorMessage::new(
			genesis_utxo,
			self.sidechain_signing_key.to_pub_key(),
			self.registration_utxo,
		)
	}

	/// Execute the registration signatures command.
	///
	/// Generates both mainchain and sidechain signatures for validator registration.
	/// Creates a registration message and signs it with both the Cardano stake pool
	/// key and the Partner Chain validator key.
	///
	/// ## Returns
	///
	/// `RegistrationCmdOutput` containing both signatures and public keys.
	///
	/// ## Output Format
	///
	/// ```json
	/// {
	///   "spo_public_key": "2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c",
	///   "spo_signature": "...",
	///   "sidechain_public_key": "02dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ec",
	///   "sidechain_signature": "..."
	/// }
	/// ```
	pub fn execute(&self) -> RegistrationCmdOutput {
		self.to_register_validator_message(self.genesis_utxo)
			.sign_and_prepare_registration_cmd_output(
				self.mainchain_signing_key.0,
				self.sidechain_signing_key.0,
			)
	}
}

/// Output structure for validator registration signatures.
///
/// Contains both mainchain and sidechain signatures with their corresponding
/// public keys. This structure provides all data required for validator
/// registration on both Cardano and Partner Chain.
///
/// ## Fields
///
/// - `spo_public_key`: Cardano stake pool operator public key (Ed25519)
/// - `spo_signature`: Mainchain signature from stake pool key
/// - `sidechain_public_key`: Partner Chain validator public key (secp256k1)
/// - `sidechain_signature`: Sidechain signature from validator key
#[derive(Clone, Debug, Serialize)]
pub struct RegistrationCmdOutput {
	/// Cardano stake pool operator public key
	pub spo_public_key: StakePoolPublicKey,
	/// Mainchain signature from Cardano stake pool key
	pub spo_signature: MainchainSignature,
	/// Partner Chain validator public key
	pub sidechain_public_key: SidechainPublicKey,
	/// Sidechain signature from Partner Chain validator key
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

/// Message structure for validator registration.
///
/// Contains the data that must be signed by both mainchain and sidechain keys
/// for validator registration. This message is encoded as a Plutus CBOR datum
/// for on-chain verification and validation.
///
/// ## CBOR Encoding
///
/// The message is automatically encoded as CBOR datum using the `ToDatum` derive macro.
/// This ensures compatibility with Cardano smart contracts and Plutus validation logic.
///
/// ## Fields
///
/// - `genesis_utxo`: Identifies the specific Partner Chain instance
/// - `sidechain_pub_key`: Validator's public key for Partner Chain operations
/// - `registration_utxo`: UTXO used for the registration transaction
#[derive(Clone, Debug, ToDatum)]
pub struct RegisterValidatorMessage {
	/// Genesis UTXO identifying the Partner Chain instance
	pub genesis_utxo: UtxoId,
	/// Partner Chain validator public key
	pub sidechain_pub_key: SidechainPublicKey,
	/// UTXO used for validator registration transaction
	pub registration_utxo: UtxoId,
}

impl RegisterValidatorMessage {
	/// Create new validator registration message.
	///
	/// Constructs a registration message from the provided parameters,
	/// converting the secp256k1 public key to the appropriate format.
	///
	/// ## Parameters
	///
	/// - `genesis_utxo`: Genesis UTXO identifying the Partner Chain
	/// - `pub_key`: secp256k1 public key of the validator
	/// - `registration_utxo`: UTXO for registration transaction
	///
	/// ## Returns
	///
	/// `RegisterValidatorMessage` ready for signature generation and CBOR encoding.
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

	/// Generate dual signatures and prepare registration output.
	///
	/// Signs the registration message with both mainchain and sidechain keys,
	/// then creates the complete output structure with signatures and public keys.
	///
	/// ## Process
	///
	/// 1. Generate Cardano stake pool signature using Ed25519 key
	/// 2. Generate Partner Chain validator signature using secp256k1 key
	/// 3. Extract corresponding public keys
	/// 4. Create output structure with all signature data
	///
	/// ## Parameters
	///
	/// - `mainchain_key`: Ed25519 signing key for Cardano operations
	/// - `sidechain_key`: secp256k1 secret key for Partner Chain operations
	///
	/// ## Returns
	///
	/// `RegistrationCmdOutput` containing both signatures and public keys
	/// ready for validator registration submission.
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
