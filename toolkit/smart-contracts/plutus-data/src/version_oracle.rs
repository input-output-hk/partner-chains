//! Plutus types for the script versioning/script caching system.
use crate::{DataDecodingError, DecodingResult, PlutusDataExtensions, decoding_error_and_log};
use cardano_serialization_lib::{BigInt, PlutusData, PlutusList};

/// Datum attached to 'VersionOraclePolicy' tokens stored on the 'VersionOracleValidator' script.
/// This datum is not versioned intentionally, as it is not subject to observation.
///
/// Original definition in the smart contracts:
/// ```haskell
/// data VersionOracleDatum = VersionOracleDatum
/// { versionOracle :: VersionOracle
/// -- ^ VersionOracle which identifies the script.
/// , currencySymbol :: CurrencySymbol
/// -- ^ Currency Symbol of the VersioningOraclePolicy tokens.
/// }
///
/// See https://preview.cexplorer.io/tx/70923772056f153d646488c56ac04d1bc2f1326f074773e4f262c63e03b72a3d/contract#data
/// for an example of transaction outputting this datum.
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct VersionOracleDatum {
	/// Id of the script in the script cache/versioning system. See [raw_scripts::ScriptId].
	pub version_oracle: u32,
	/// Script hash of the oracle policy where cached/versioned scripts are stored.
	pub currency_symbol: [u8; 28],
}

impl TryFrom<PlutusData> for VersionOracleDatum {
	type Error = DataDecodingError;
	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		datum
			.as_list()
			.filter(|datum| datum.len() == 2)
			.and_then(|items| {
				Some(VersionOracleDatum {
					version_oracle: items.get(0).as_u32()?,
					currency_symbol: items.get(1).as_bytes()?.try_into().ok()?,
				})
			})
			.ok_or_else(|| {
				decoding_error_and_log(&datum, "VersionOracleDatum", "Expected [u32, [u8;32]]")
			})
	}
}

impl From<VersionOracleDatum> for PlutusData {
	fn from(datum: VersionOracleDatum) -> Self {
		PlutusData::new_list(&{
			let mut list = PlutusList::new();
			list.add(&PlutusData::new_integer(&BigInt::from(datum.version_oracle)));
			list.add(&PlutusData::new_bytes(datum.currency_symbol.to_vec()));
			list
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_helpers::*;
	use hex_literal::hex;
	use pretty_assertions::assert_eq;

	#[test]
	fn decoding() {
		let plutus_data = test_plutus_data!({"list": [
			{"int": 32},
			{"bytes": "e50a076eed80e645499abc26a5b33b61bef32f8cb1ab29b1ffcc1b88"}
		]});

		let expected_datum = VersionOracleDatum {
			version_oracle: 32,
			currency_symbol: hex!("e50a076eed80e645499abc26a5b33b61bef32f8cb1ab29b1ffcc1b88"),
		};

		assert_eq!(VersionOracleDatum::try_from(plutus_data).unwrap(), expected_datum)
	}

	#[test]
	fn encoding() {
		let datum = VersionOracleDatum {
			version_oracle: 32,
			currency_symbol: hex!("e50a076eed80e645499abc26a5b33b61bef32f8cb1ab29b1ffcc1b88"),
		};

		let expected_plutus_data = test_plutus_data!({"list": [
			{"int": 32},
			{"bytes": "e50a076eed80e645499abc26a5b33b61bef32f8cb1ab29b1ffcc1b88"}
		]});

		assert_eq!(PlutusData::from(datum), expected_plutus_data)
	}
}
