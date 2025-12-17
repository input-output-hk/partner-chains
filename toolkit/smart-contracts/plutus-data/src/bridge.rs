//! Plutus data types used by the token bridge

use crate::*;
use cardano_serialization_lib::{JsError, MetadataMap, TransactionMetadatum};
use sidechain_domain::byte_string::ByteString;

/// Datum containing token transfer data
#[derive(Clone, Debug, PartialEq)]
pub enum TokenTransferMetadatum {
	/// Version 1
	V1(TokenTransferMetadatumV1),
}

impl TokenTransferMetadatum {
	/// Creates v1 reserve transfer metadatum
	pub fn reserve_v1() -> Self {
		Self::V1(TokenTransferMetadatumV1::ReserveTransfer)
	}

	/// Creates v1 user transfer metadatum
	pub fn user_v1(receiver: ByteString) -> Self {
		Self::V1(TokenTransferMetadatumV1::UserTransfer { receiver })
	}
}

impl VersionedMetadatum for TokenTransferMetadatum {
	fn decode_version(version: i32, payload: TransactionMetadatum) -> Result<Self, JsError> {
		match version {
			1 => Ok(Self::V1(TokenTransferMetadatumV1::decode_payload(payload)?)),
			_ => Err(JsError::from_str(&format!(
				"Unsupported TokenTransferMetadatum version {version}"
			))),
		}
	}

	fn version(&self) -> i32 {
		match self {
			Self::V1(_) => 1,
		}
	}

	fn encode_payload(&self) -> Result<TransactionMetadatum, JsError> {
		match self {
			Self::V1(v1) => v1.encode_payload(),
		}
	}
}

/// Datum containing token transfer data, version 1
#[derive(Clone, Debug, PartialEq)]
pub enum TokenTransferMetadatumV1 {
	/// User-initiated transfer sent to a specific receiver address
	UserTransfer {
		/// Receiving address on the Partner Chain
		receiver: ByteString,
	},
	/// Reserve transfer
	ReserveTransfer,
}

impl TokenTransferMetadatumV1 {
	fn decode_payload(value: TransactionMetadatum) -> Result<Self, JsError> {
		if let Ok("reserve") = value.as_text().as_deref() {
			return Ok(Self::ReserveTransfer);
		}

		Ok(Self::UserTransfer {
			receiver: ByteString(value.as_map()?.get_str("receiver")?.as_bytes()?),
		})
	}

	fn encode_payload(&self) -> Result<TransactionMetadatum, JsError> {
		match self {
			TokenTransferMetadatumV1::ReserveTransfer => {
				Ok(TransactionMetadatum::new_text("reserve".to_string())?)
			},
			TokenTransferMetadatumV1::UserTransfer { receiver } => {
				let receiver_label = TransactionMetadatum::new_text("receiver".to_string())?;
				let receiver_bytes = TransactionMetadatum::new_bytes(receiver.to_vec())?;

				let mut map = MetadataMap::new();
				map.insert(&receiver_label, &receiver_bytes);
				Ok(TransactionMetadatum::new_map(&map))
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_helpers::test_tx_metadata;
	use cardano_serialization_lib::TransactionMetadatum;

	fn reserve_transfer_metadata() -> TransactionMetadatum {
		test_tx_metadata!({
			"v": 1,
			"p": "reserve",
		})
	}

	fn user_transfer_data(addr: &[u8]) -> TransactionMetadatum {
		test_tx_metadata!({
			"v": 1,
			"p": {
				"receiver": "0x".to_owned() + &hex::encode(addr)
			}

		})
	}

	mod decode {
		use super::*;
		use hex_literal::hex;
		use pretty_assertions::assert_eq;

		#[test]
		fn user_transfer_v1() {
			let datum = TokenTransferMetadatum::decode(user_transfer_data(&hex!("abcd")))
				.expect("Should decode successfully");
			assert_eq!(
				datum,
				TokenTransferMetadatum::V1(TokenTransferMetadatumV1::UserTransfer {
					receiver: ByteString(hex!("abcd").into())
				})
			)
		}

		#[test]
		fn reserve_transfer_v1() {
			let datum = TokenTransferMetadatum::decode(reserve_transfer_metadata())
				.expect("Should decode successfully");
			assert_eq!(datum, TokenTransferMetadatum::V1(TokenTransferMetadatumV1::ReserveTransfer))
		}
	}

	mod encode {
		use super::*;
		use hex_literal::hex;
		use pretty_assertions::assert_eq;

		#[test]
		fn user_transfer_v1() {
			let data: TransactionMetadatum =
				TokenTransferMetadatum::V1(TokenTransferMetadatumV1::UserTransfer {
					receiver: ByteString::from_hex_unsafe("abcd"),
				})
				.encode()
				.expect("Should succeed");

			assert_eq!(data, user_transfer_data(&hex!("abcd")))
		}

		#[test]
		fn reserve_transfer_v1() {
			let data: TransactionMetadatum =
				TokenTransferMetadatum::V1(TokenTransferMetadatumV1::ReserveTransfer)
					.encode()
					.expect("Should succeed");

			assert_eq!(data, reserve_transfer_metadata())
		}
	}
}
