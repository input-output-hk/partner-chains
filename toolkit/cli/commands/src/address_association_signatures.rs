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

/// Generates Ed25519 signatures to associate Cardano stake addresses with Partner Chain addresses.
/// Generic over address type to support different Partner Chain implementations.
#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct AddressAssociationSignaturesCmd<
	PartnerchainAddress: Clone + Sync + Send + FromStr + 'static,
> {
	/// Genesis UTXO that identifies the target Partner Chain
	#[arg(long)]
	pub genesis_utxo: UtxoId,
	/// Partner Chain address to be associated with the Cardano stake address
	#[arg(long, value_parser=parse_pc_address::<PartnerchainAddress>)]
	pub partnerchain_address: PartnerchainAddress,
	/// Ed25519 signing key for the Cardano stake address. Its public key will be associated with partnerchain_address.
	#[arg(long)]
	pub signing_key: StakeSigningKeyParam,
}

/// Parses Partner Chain address from string format.
fn parse_pc_address<T: FromStr>(s: &str) -> Result<T, String> {
	T::from_str(s).map_err(|_| "Failed to parse Partner Chain address".to_owned())
}

impl<PartnerchainAddress> AddressAssociationSignaturesCmd<PartnerchainAddress>
where
	PartnerchainAddress: Serialize + Clone + Sync + Send + FromStr + Encode + 'static,
{
	/// Generates signature and outputs JSON to stdout.
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

	/// Signs SCALE-encoded address association message with Ed25519.
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
