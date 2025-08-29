//! Plutus data types used by the token bridge

use crate::*;
use cardano_serialization_lib::{PlutusData, traits::NoneOrEmpty};
use sidechain_domain::byte_string::ByteString;

/// Datum containing token transfer data
#[derive(Clone, Debug, PartialEq)]
pub enum TokenTransferDatum {
	/// Version 1
	V1(TokenTransferDatumV1),
}

/// Datum containing token transfer data, version 1
#[derive(Clone, Debug, PartialEq)]
pub enum TokenTransferDatumV1 {
	/// User-initiated transfer sent to a specific receiver address
	UserTransfer {
		/// Receiving address on the Partner Chain
		receiver: ByteString,
	},
	/// Reserve transfer
	ReserveTransfer,
}

impl From<TokenTransferDatumV1> for PlutusData {
	fn from(datum: TokenTransferDatumV1) -> Self {
		VersionedGenericDatum {
			version: 1,
			datum: PlutusData::new_empty_constr_plutus_data(&0u64.into()),
			appendix: {
				match datum {
					TokenTransferDatumV1::UserTransfer { receiver } => {
						PlutusData::new_single_value_constr_plutus_data(
							&0u64.into(),
							&PlutusData::new_bytes(receiver.0),
						)
					},
					TokenTransferDatumV1::ReserveTransfer => {
						PlutusData::new_empty_constr_plutus_data(&1u64.into())
					},
				}
			},
		}
		.into()
	}
}

impl From<TokenTransferDatum> for PlutusData {
	fn from(datum: TokenTransferDatum) -> Self {
		match datum {
			TokenTransferDatum::V1(datum) => datum.into(),
		}
	}
}

impl TryFrom<PlutusData> for TokenTransferDatum {
	type Error = DataDecodingError;
	fn try_from(data: PlutusData) -> Result<Self, Self::Error> {
		Self::decode(&data)
	}
}

impl VersionedDatum for TokenTransferDatum {
	fn decode(data: &PlutusData) -> crate::DecodingResult<Self> {
		match plutus_data_version_and_payload(data) {
			None => Err(decoding_error_and_log(data, "TokenTransferDatum", "unversioned datum")),
			Some(VersionedGenericDatum { appendix, version: 1, .. }) => {
				decode_v1_token_transfer_datum(&appendix).ok_or_else(|| {
					decoding_error_and_log(&appendix, "TokenTransferDatum", "malformed appendix")
				})
			},
			Some(_) => Err(decoding_error_and_log(data, "TokenTransferDatum", "invalid version")),
		}
	}
}

fn decode_v1_token_transfer_datum(appendix: &PlutusData) -> Option<TokenTransferDatum> {
	println!(">> {appendix:?}");
	let constr = appendix.as_constr_plutus_data()?;
	let alternative = u64::from(constr.alternative());
	let data = constr.data();

	match alternative {
		0 if data.len() == 1 => {
			let receiver = data.get(0).as_bytes()?.into();
			Some(TokenTransferDatum::V1(TokenTransferDatumV1::UserTransfer { receiver }))
		},
		1 if data.is_none_or_empty() => {
			Some(TokenTransferDatum::V1(TokenTransferDatumV1::ReserveTransfer))
		},
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_helpers::test_plutus_data;

	fn reserve_transfer_data() -> PlutusData {
		test_plutus_data!({
			"list": [
				{ "constructor": 0, "fields": [] },
				{ "constructor": 1, "fields": [] },
				{ "int":1 },

			]
		})
	}

	fn user_transfer_data(addr: &[u8]) -> PlutusData {
		test_plutus_data!({
					"list": [
						{ "constructor": 0, "fields": [] },
						{ "constructor": 0, "fields": [{ "bytes": hex::encode(addr) }] },
						{ "int":1 },

					]
		})
	}

	mod decode {
		use super::*;
		use hex_literal::hex;
		use pretty_assertions::assert_eq;

		#[test]
		fn user_transfer_v1() {
			let datum = TokenTransferDatum::decode(&user_transfer_data(&hex!("abcd")))
				.expect("Should decode successfully");
			assert_eq!(
				datum,
				TokenTransferDatum::V1(TokenTransferDatumV1::UserTransfer {
					receiver: ByteString(hex!("abcd").into())
				})
			)
		}

		#[test]
		fn reserve_transfer_v1() {
			let datum = TokenTransferDatum::decode(&reserve_transfer_data())
				.expect("Should decode successfully");
			assert_eq!(datum, TokenTransferDatum::V1(TokenTransferDatumV1::ReserveTransfer))
		}
	}

	mod encode {
		use super::*;
		use hex_literal::hex;
		use pretty_assertions::assert_eq;

		#[test]
		fn user_transfer_v1() {
			let data: PlutusData = TokenTransferDatum::V1(TokenTransferDatumV1::UserTransfer {
				receiver: ByteString::from_hex_unsafe("abcd"),
			})
			.into();

			assert_eq!(data, user_transfer_data(&hex!("abcd")))
		}

		#[test]
		fn reserve_transfer_v1() {
			let data: PlutusData =
				TokenTransferDatum::V1(TokenTransferDatumV1::ReserveTransfer).into();

			assert_eq!(data, reserve_transfer_data())
		}
	}
}
