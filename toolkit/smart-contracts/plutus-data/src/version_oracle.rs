//! Plutus types for the script versioning/script caching system.
use crate::{
	DataDecodingError, DecodingResult, PlutusDataExtensions, ScriptHash, decoding_error_and_log,
};
use cardano_serialization_lib::{BigInt, BigNum, ConstrPlutusData, PlutusData, PlutusList};

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
/// ```
///
/// See https://preview.cexplorer.io/tx/70923772056f153d646488c56ac04d1bc2f1326f074773e4f262c63e03b72a3d/contract#data
/// for an example of transaction outputting this datum.
#[derive(Clone, Debug, PartialEq)]
pub struct VersionOracleDatum {
	/// 'VersionOracle' of the script.
	pub version_oracle: VersionOracle,
	/// Script hash of the oracle policy where cached/versioned scripts are stored.
	pub currency_symbol: ScriptHash,
}

impl TryFrom<PlutusData> for VersionOracleDatum {
	type Error = DataDecodingError;
	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		datum
			.as_list()
			.filter(|datum| datum.len() == 2)
			.and_then(|items| {
				Some(VersionOracleDatum {
					version_oracle: items.get(0).try_into().ok()?,
					currency_symbol: items.get(1).try_into().ok()?,
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
			list.add(&datum.version_oracle.into());
			list.add(&datum.currency_symbol.into());
			list
		})
	}
}

/// Original definition in the smart contracts:
/// ```haskell
/// newtype VersionOracle = VersionOracle
///   { scriptId :: Integer
///   -- ^ Unique identifier of the validator.
///   }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct VersionOracle {
	/// Id of the script in the script cache/versioning system. See [raw_scripts::ScriptId].
	pub script_id: u32,
}

impl TryFrom<PlutusData> for VersionOracle {
	type Error = DataDecodingError;
	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		datum
			.as_u32()
			.and_then(|script_id| Some(VersionOracle { script_id }))
			.ok_or_else(|| decoding_error_and_log(&datum, "VersionOracle", "Expected u32"))
	}
}

impl From<VersionOracle> for PlutusData {
	fn from(datum: VersionOracle) -> Self {
		PlutusData::new_integer(&BigInt::from(datum.script_id as u32))
	}
}

impl From<raw_scripts::ScriptId> for VersionOracle {
	fn from(script_id: raw_scripts::ScriptId) -> Self {
		VersionOracle { script_id: script_id as u32 }
	}
}

/// Redeemer type for VersioningOracle minting policy.
///
/// Original definition in the smart contracts:
/// ```haskell
/// data VersionOraclePolicyRedeemer
///   = -- | Mint initial versioning token. Used during Partner Chain initialization.
///     InitializeVersionOracle VersionOracle ScriptHash
///   | -- | Mint a new versioning token ensuring it contains correct datum and
///     -- reference script.
///     MintVersionOracle VersionOracle ScriptHash
///   | -- | Burn existing versioning token.
///     BurnVersionOracle VersionOracle
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum VersionOraclePolicyRedeemer {
	/// Mint initial versioning token. Used during Partner Chain initialization.
	InitializeVersionOracle(VersionOracle, ScriptHash),
	/// Mint a new versioning token ensuring it contains correct datum and
	MintVersionOracle(VersionOracle, ScriptHash),
	/// Burn existing versioning token.
	BurnVersionOracle(VersionOracle),
}

impl TryFrom<PlutusData> for VersionOraclePolicyRedeemer {
	type Error = DataDecodingError;
	fn try_from(redeemer: PlutusData) -> DecodingResult<Self> {
		redeemer
			.as_constr_plutus_data()
			.and_then(|constr| match From::<BigNum>::from(constr.alternative()) {
				0u64 if constr.data().len() == 2 => {
					Some(VersionOraclePolicyRedeemer::InitializeVersionOracle(
						constr.data().get(0).try_into().ok()?,
						constr.data().get(1).try_into().ok()?,
					))
				},
				1u64 if constr.data().len() == 2 => {
					Some(VersionOraclePolicyRedeemer::MintVersionOracle(
						constr.data().get(0).try_into().ok()?,
						constr.data().get(1).try_into().ok()?,
					))
				},
				2u64 if constr.data().len() == 1 => {
					Some(VersionOraclePolicyRedeemer::BurnVersionOracle(
						constr.data().get(0).try_into().ok()?,
					))
				},
				_ => None,
			})
			.ok_or_else(|| {
				decoding_error_and_log(
					&redeemer,
					"VersionOraclePolicyRedeemer",
					"Expected one of Constr 0 [u32, [u8;32]], Constr 1 [u32, [u8;32]], or Constr 2 [u32]",
				)
			})
	}
}

impl From<VersionOraclePolicyRedeemer> for PlutusData {
	fn from(redeemer: VersionOraclePolicyRedeemer) -> Self {
		match redeemer {
			VersionOraclePolicyRedeemer::InitializeVersionOracle(version_oracle, script_hash) => {
				PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(&0u64.into(), &{
					let mut list = PlutusList::new();
					list.add(&version_oracle.into());
					list.add(&script_hash.into());
					list
				}))
			},
			VersionOraclePolicyRedeemer::MintVersionOracle(version_oracle, script_hash) => {
				PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(&1u64.into(), &{
					let mut list = PlutusList::new();
					list.add(&version_oracle.into());
					list.add(&script_hash.into());
					list
				}))
			},
			VersionOraclePolicyRedeemer::BurnVersionOracle(version_oracle) => {
				PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(&2u64.into(), &{
					let mut list = PlutusList::new();
					list.add(&version_oracle.into());
					list
				}))
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_helpers::*;
	use hex_literal::hex;
	use pretty_assertions::assert_eq;
	use raw_scripts::ScriptId;

	#[test]
	fn decoding() {
		let plutus_data = test_plutus_data!({"list": [
			{"int": 32},
			{"bytes": "e50a076eed80e645499abc26a5b33b61bef32f8cb1ab29b1ffcc1b88"}
		]});

		let expected_datum = VersionOracleDatum {
			version_oracle: VersionOracle { script_id: ScriptId::GovernancePolicy as u32 },
			currency_symbol: ScriptHash(hex!(
				"e50a076eed80e645499abc26a5b33b61bef32f8cb1ab29b1ffcc1b88"
			)),
		};

		assert_eq!(VersionOracleDatum::try_from(plutus_data).unwrap(), expected_datum)
	}

	#[test]
	fn encoding() {
		let datum = VersionOracleDatum {
			version_oracle: VersionOracle { script_id: ScriptId::GovernancePolicy as u32 },
			currency_symbol: ScriptHash(hex!(
				"e50a076eed80e645499abc26a5b33b61bef32f8cb1ab29b1ffcc1b88"
			)),
		};

		let expected_plutus_data = test_plutus_data!({"list": [
			{"int": 32},
			{"bytes": "e50a076eed80e645499abc26a5b33b61bef32f8cb1ab29b1ffcc1b88"}
		]});

		assert_eq!(PlutusData::from(datum), expected_plutus_data)
	}
}
