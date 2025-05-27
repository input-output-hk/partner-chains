//! # Address Association Signatures
//!
//! Generate cryptographic signatures for associating Cardano stake addresses with Partner Chain addresses.
//! This module enables stake pool operators to link their Cardano identity with Partner Chain participation.
//!
//! ## Process Overview
//!
//! 1. Create message containing Partner Chain address, stake public key, and genesis UTXO
//! 2. Sign message using Cardano stake signing key (Ed25519)
//! 3. Output signature, public key, and Partner Chain address in JSON format
//!
//! ## CLI Integration
//!
//! ```bash
//! partner-chains-cli address-association-signatures \
//!   --genesis-utxo 59104061ffa0d66f9ba0135d6fc6a884a395b10f8ae9cb276fc2c3bfdfedc260#1 \
//!   --partnerchain-address d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d \
//!   --signing-key d75c630516c33a66b11b3444a70b65083aeb21353bd919cc5e3daa02c9732a84
//! ```
//!
//! ## Output Format
//!
//! ```json
//! {
//!   "partnerchain_address": "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
//!   "signature": "1aa8c1b363a207ddadf0c6242a0632f5a557690a327d0245f9d473b983b3d8e1c95a3dd804cab41123c36ddbcb7137b8261c35d5c8ef04ce9d0f8d5c4b3ca607",
//!   "stake_public_key": "2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c"
//! }
//! ```

use crate::key_params::StakeSigningKeyParam;
use anyhow::Ok;
use byte_string::ByteString;
use clap::Parser;
use pallet_address_associations::AddressAssociationSignedMessage;
use parity_scale_codec::Encode;
use serde::Serialize;
use serde_json::json;
use sidechain_domain::*;
use std::str::FromStr;

/// Command for generating address association signatures.
///
/// Associates a Cardano stake address with a Partner Chain address through cryptographic signatures.
/// The resulting signature proves ownership of the Cardano stake key and authorizes the address association.
///
/// ## Parameters
///
/// - `genesis_utxo`: Identifies the target Partner Chain
/// - `partnerchain_address`: Partner Chain address to associate with Cardano stake key
/// - `signing_key`: Cardano stake signing key in hex format
///
/// ## Example Usage
///
/// ```bash
/// partner-chains-cli address-association-signatures \
///   --genesis-utxo 59104061ffa0d66f9ba0135d6fc6a884a395b10f8ae9cb276fc2c3bfdfedc260#1 \
///   --partnerchain-address d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d \
///   --signing-key d75c630516c33a66b11b3444a70b65083aeb21353bd919cc5e3daa02c9732a84
/// ```
#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct AddressAssociationSignaturesCmd<
	PartnerchainAddress: Clone + Sync + Send + FromStr + 'static,
> {
	/// Genesis UTXO of the target Partner Chain
	#[arg(long)]
	pub genesis_utxo: UtxoId,
	/// Partner Chain address to be associated with the Cardano address
	#[arg(long, value_parser=parse_pc_address::<PartnerchainAddress>)]
	pub partnerchain_address: PartnerchainAddress,
	/// Cardano Stake Signing Key bytes in hex format. Its public key will be associated with partnerchain_address.
	#[arg(long)]
	pub signing_key: StakeSigningKeyParam,
}

/// Parse Partner Chain address from string input.
///
/// Converts string representation to the target address type.
/// Returns error message if parsing fails.
fn parse_pc_address<T: FromStr>(s: &str) -> Result<T, String> {
	T::from_str(s).map_err(|_| "Failed to parse Partner Chain address".to_owned())
}

impl<PartnerchainAddress> AddressAssociationSignaturesCmd<PartnerchainAddress>
where
	PartnerchainAddress: Serialize + Clone + Sync + Send + FromStr + Encode + 'static,
{
	/// Execute the address association signature command.
	///
	/// Generates a cryptographic signature linking the Cardano stake key with the Partner Chain address.
	/// Outputs the result as formatted JSON containing the signature, public key, and Partner Chain address.
	///
	/// ## Output Format
	///
	/// ```json
	/// {
	///   "partnerchain_address": "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
	///   "signature": "1aa8c1b363a207ddadf0c6242a0632f5a557690a327d0245f9d473b983b3d8e1c95a3dd804cab41123c36ddbcb7137b8261c35d5c8ef04ce9d0f8d5c4b3ca607",
	///   "stake_public_key": "2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c"
	/// }
	/// ```
	///
	/// ## Errors
	///
	/// Returns `anyhow::Error` if JSON serialization fails.
	pub fn execute(&self) -> anyhow::Result<()> {
		let signature = self.sign();
		let output = json!({
			"partnerchain_address": self.partnerchain_address,
			"signature": signature,
			"stake_public_key": self.signing_key.vkey()

		});
		println!("{}", serde_json::to_string_pretty(&output)?);
		Ok(())
	}

	/// Generate cryptographic signature for address association.
	///
	/// Creates an `AddressAssociationSignedMessage` containing the stake public key,
	/// Partner Chain address, and genesis UTXO. Signs the SCALE-encoded message
	/// using the Ed25519 stake signing key.
	///
	/// ## Process
	///
	/// 1. Construct message with stake public key, Partner Chain address, and genesis UTXO
	/// 2. Encode message using SCALE codec
	/// 3. Sign encoded bytes with Ed25519 stake signing key
	/// 4. Return signature as ByteString
	///
	/// ## Returns
	///
	/// `ByteString` containing the Ed25519 signature bytes.
	fn sign(&self) -> ByteString {
		let msg = AddressAssociationSignedMessage {
			stake_public_key: self.signing_key.vkey(),
			partnerchain_address: self.partnerchain_address.clone(),
			genesis_utxo: self.genesis_utxo,
		};
		let encoded = msg.encode();
		self.signing_key.0.sign(&encoded).to_bytes().to_vec().into()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use hex::FromHexError;
	use hex_literal::hex;
	use sidechain_domain::byte_string::ByteString;

	#[derive(Clone, Encode, Serialize)]
	struct AccountId32(pub [u8; 32]);

	impl FromStr for AccountId32 {
		type Err = anyhow::Error;

		fn from_str(s: &str) -> Result<Self, Self::Err> {
			let bytes =
				hex::decode(s)?.try_into().map_err(|_| FromHexError::InvalidStringLength)?;

			Ok(Self(bytes))
		}
	}

	// This test is specifically kept in sync with the pallet signature verification test
	#[test]
	fn signature_test() {
		let cmd = AddressAssociationSignaturesCmd {
			genesis_utxo: UtxoId::new(
				hex!("59104061ffa0d66f9ba0135d6fc6a884a395b10f8ae9cb276fc2c3bfdfedc260"),
				1,
			),
			partnerchain_address:
				// re-encoding of 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY (Alice)
				AccountId32(hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d")),
			signing_key: StakeSigningKeyParam::from_str(
				// Private key of Alice (pubkey: 2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c)
				"d75c630516c33a66b11b3444a70b65083aeb21353bd919cc5e3daa02c9732a84"
			).unwrap(),
		};

		assert_eq!(
			cmd.sign(),
			ByteString(hex!("1aa8c1b363a207ddadf0c6242a0632f5a557690a327d0245f9d473b983b3d8e1c95a3dd804cab41123c36ddbcb7137b8261c35d5c8ef04ce9d0f8d5c4b3ca607").into())
		);
	}
}
