use anyhow::anyhow;
use cardano_serialization_lib::{PrivateKey, PublicKey};
use sidechain_domain::MainchainKeyHash;

/// Signing (payment) key abstraction layer. Hides internal crypto library details.
/// It is either:
/// * 32 bytes regular private key
/// * 64 bytes extended private key
pub struct CardanoPaymentSigningKey(pub(crate) PrivateKey);

impl CardanoPaymentSigningKey {
	/// Constructs [CardanoPaymentSigningKey] from 128 byte extended payment signing key.
	/// The 128 bytes of the key are: 64 byte prefix, 32 byte verification key, 32 byte chain code
	pub fn from_extended_128_bytes(bytes: [u8; 128]) -> anyhow::Result<Self> {
		let prefix: [u8; 64] = bytes[0..64].try_into().unwrap();
		Ok(Self(
			PrivateKey::from_extended_bytes(&prefix)
				.map_err(|e| anyhow!("Couldn't parse 128 bytes into a BIP32 Private Key: {e}"))?,
		))
	}

	/// Constructs [CardanoPaymentSigningKey] from 32 byte payment signing key.
	pub fn from_normal_bytes(bytes: [u8; 32]) -> Result<Self, anyhow::Error> {
		Ok(Self(
			PrivateKey::from_normal_bytes(&bytes)
				.map_err(|e| anyhow!("Couldn't parse 32 bytes into a Private Key: {e}"))?,
		))
	}

	/// Signs `message` with the [CardanoPaymentSigningKey].
	pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
		Ok(self.0.sign(message).to_bytes())
	}

	/// Hashes [CardanoPaymentSigningKey] to domain type [MainchainKeyHash].
	pub fn to_pub_key_hash(&self) -> MainchainKeyHash {
		MainchainKeyHash(
			self.0
				.to_public()
				.hash()
				.to_bytes()
				.as_slice()
				.try_into()
				.expect("CSL PublicKeyHash is 28 bytes"),
		)
	}

	/// Converts [CardanoPaymentSigningKey] to CSL [PublicKey].
	pub fn to_csl_pub_key(&self) -> PublicKey {
		self.0.to_public()
	}

	/// Returns raw bytes of [CardanoPaymentSigningKey].
	pub fn to_bytes(&self) -> Vec<u8> {
		self.0.as_bytes()
	}
}

impl TryFrom<CardanoKeyFileContent> for CardanoPaymentSigningKey {
	type Error = anyhow::Error;

	fn try_from(key: CardanoKeyFileContent) -> Result<Self, Self::Error> {
		let key_type = key.r#type.clone();
		if key_type == "PaymentSigningKeyShelley_ed25519" {
			Ok(CardanoPaymentSigningKey::from_normal_bytes(key.raw_key_bytes()?)?)
		} else if key_type == "PaymentExtendedSigningKeyShelley_ed25519_bip32" {
			Ok(CardanoPaymentSigningKey::from_extended_128_bytes(key.raw_key_bytes()?)?)
		} else {
			Err(anyhow!("Unsupported key type: {}. Expected a signing key", key_type))
		}
	}
}

impl Clone for CardanoPaymentSigningKey {
	fn clone(&self) -> Self {
		let bytes = self.0.as_bytes();
		let private_key = if bytes.len() == 32 {
			PrivateKey::from_normal_bytes(&bytes)
				.expect("PrivateKey bytes are valid to clone the key")
		} else {
			PrivateKey::from_extended_bytes(&bytes)
				.expect("PrivateKey bytes are valid to clone the key")
		};
		Self(private_key)
	}
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// Type representing Cardano key file.
///
/// Note: Field names are used for serialization, do not rename.
pub struct CardanoKeyFileContent {
	/// Type of the Cardano key.
	pub r#type: String,
	/// CBOR hex of the key.
	pub cbor_hex: String,
}

impl CardanoKeyFileContent {
	/// Parses file to [CardanoKeyFileContent].
	pub fn parse_file(path: &str) -> anyhow::Result<Self> {
		let file_content = std::fs::read_to_string(path)
			.map_err(|e| anyhow!("Could not read Cardano key file at: {path}. Cause: {e}"))?;
		serde_json::from_str::<Self>(&file_content)
			.map_err(|e| anyhow!("{path} is not valid Cardano key JSON file. {e}"))
	}
	/// Parses raw bytes of 'cborHex' field of Cardano key file content.
	/// Works for 32, 64 and 128 bytes hex strings (68, 132, 260 hex digits) - assumes CBOR prefix has 4 hex digits.
	pub fn raw_key_bytes<const N: usize>(&self) -> anyhow::Result<[u8; N]> {
		let (_cbor_prefix, vkey) = self.cbor_hex.split_at(4);
		let bytes: [u8; N] = hex::decode(vkey)
			.map_err(|err| {
				anyhow!(
					"Invalid cborHex value of Cardano key - not valid hex: {}\n{err:?}",
					self.cbor_hex
				)
			})?
			.try_into()
			.map_err(|_| {
				anyhow!(
					"Invalid cborHex value of Cardano key - incorrect length: {}",
					self.cbor_hex
				)
			})?;
		Ok(bytes)
	}
}

#[cfg(test)]
mod tests {
	use crate::cardano_keys::{CardanoKeyFileContent, CardanoPaymentSigningKey};

	#[test]
	fn cardano_key_content_key_raw_bytes() {
		let key_file_content = CardanoKeyFileContent {
			r#type: "PaymentSigningKeyShelley_ed25519".to_owned(),
			cbor_hex: "5820d0a6c5c921266d15dc8d1ce1e51a01e929a686ed3ec1a9be1145727c224bf386"
				.to_owned(),
		};
		assert!(key_file_content.raw_key_bytes::<31>().is_err());
		assert_eq!(
			hex::encode(key_file_content.raw_key_bytes::<32>().unwrap()),
			"d0a6c5c921266d15dc8d1ce1e51a01e929a686ed3ec1a9be1145727c224bf386"
		);
		assert!(key_file_content.raw_key_bytes::<33>().is_err());
	}

	#[test]
	fn signing_key_from_extended() {
		let key_file_content = CardanoKeyFileContent {
			r#type: "PaymentExtendedSigningKeyShelley_ed25519_bip32".to_owned(),
			cbor_hex: "588020baf85b0e955e969cdaa852b31f223bad0348c274790c2a924602efdaba144266994eeb10f17618065431db154d28c0c7ce11277f412d614ebe82c59688b0244fbd942f1a7b94da07dfcf1c8be9826fd6222c0bae8604eebe0b6215f5d9b841203e23e0617b7e5191898dba700a7541152a3e03a816fc61b3887fe85c6d37d1".to_owned()
		};
		let key = CardanoPaymentSigningKey::try_from(key_file_content).unwrap();
		assert_eq!(
			hex::encode(key.to_pub_key_hash().0),
			"9e287cbfac63670ff624edc69eea5f26c6f56f86f474e3e0d83f7c5c"
		);
	}

	#[test]
	fn signing_key_from_normal() {
		let key_file_content = CardanoKeyFileContent {
			r#type: "PaymentSigningKeyShelley_ed25519".to_owned(),
			cbor_hex: "5820d0a6c5c921266d15dc8d1ce1e51a01e929a686ed3ec1a9be1145727c224bf386"
				.to_owned(),
		};
		let key = CardanoPaymentSigningKey::try_from(key_file_content).unwrap();
		assert_eq!(
			hex::encode(key.to_pub_key_hash().0),
			"e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"
		);
	}
}
