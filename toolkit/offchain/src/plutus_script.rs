use anyhow::{anyhow, Context};
use cardano_serialization_lib::{
	Address, Language, LanguageKind, NetworkIdKind, PlutusData, ScriptHash,
};
use ogmios_client::types::{OgmiosScript, OgmiosScript::Plutus};
use plutus::ToDatum;
use sidechain_domain::PolicyId;
use uplc::ast::{DeBruijn, Program};

use crate::{csl::*, untyped_plutus::*};

/// Wraps a Plutus script cbor
#[derive(Clone, PartialEq, Eq)]
pub struct PlutusScript {
	pub bytes: Vec<u8>,
	pub language: Language,
}

impl std::fmt::Debug for PlutusScript {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PlutusScript")
			.field("bytes", &hex::encode(&self.bytes))
			.field("language", &self.language.kind())
			.finish()
	}
}

impl PlutusScript {
	pub fn from_cbor(cbor: &[u8], language: Language) -> Self {
		Self { bytes: cbor.into(), language }
	}

	pub fn from_ogmios(ogmios_script: OgmiosScript) -> anyhow::Result<Self> {
		if let Plutus(script) = ogmios_script {
			let language_kind = match script.language.as_str() {
				"plutus:v1" => Language::new_plutus_v1(),
				"plutus:v2" => Language::new_plutus_v2(),
				"plutus:v3" => Language::new_plutus_v3(),
				_ => {
					return Err(anyhow!(
						"Unsupported Plutus language version: {}",
						script.language
					));
				},
			};
			Ok(Self { bytes: script.cbor, language: language_kind })
		} else {
			Err(anyhow!("Expected Plutus script, got something else."))
		}
	}

	/// This function is needed to create [PlutusScript] from scripts in [raw_scripts],
	/// which are encoded as a cbor byte string containing the cbor of the script
	/// itself. This function removes this layer of wrapping.
	pub fn from_wrapped_cbor(cbor: &[u8], language: Language) -> anyhow::Result<Self> {
		Ok(Self::from_cbor(&unwrap_one_layer_of_cbor(cbor)?, language))
	}

	pub fn apply_data(self, data: impl ToDatum) -> Result<Self, anyhow::Error> {
		let data = datum_to_uplc_plutus_data(&data.to_datum());
		self.apply_uplc_data(data)
	}

	pub fn apply_uplc_data(self, data: uplc::PlutusData) -> Result<Self, anyhow::Error> {
		let mut buffer = Vec::new();
		let mut program = Program::<DeBruijn>::from_cbor(&self.bytes, &mut buffer)
			.map_err(|e| anyhow!(e.to_string()))?;
		program = program.apply_data(data);
		let bytes = program
			.to_cbor()
			.map_err(|_| anyhow!("Couldn't encode resulting script as CBOR."))?;
		Ok(Self { bytes, ..self })
	}

	/// Builds an CSL `Address` for plutus script from the data obtained from smart contracts.
	pub fn address(&self, network: NetworkIdKind) -> Address {
		script_address(&self.bytes, network, self.language)
	}

	// Returns PlutusData representation of the given script. It is done in the same way as on-chain code expects.
	// First, the Address is created, then it is converted to PlutusData.
	pub fn address_data(&self, network: NetworkIdKind) -> anyhow::Result<uplc::PlutusData> {
		csl_plutus_data_to_uplc(&PlutusData::from_address(&self.address(network))?)
	}

	/// Returns bech32 address of the given PlutusV2 script
	pub fn address_bech32(&self, network: NetworkIdKind) -> anyhow::Result<String> {
		self.address(network)
			.to_bech32(None)
			.context("Converting script address to bech32")
	}

	pub fn script_hash(&self) -> [u8; 28] {
		plutus_script_hash(&self.bytes, self.language)
	}

	pub fn csl_script_hash(&self) -> ScriptHash {
		ScriptHash::from(self.script_hash())
	}

	pub fn policy_id(&self) -> PolicyId {
		PolicyId(self.script_hash())
	}

	pub fn to_csl(&self) -> cardano_serialization_lib::PlutusScript {
		match self.language.kind() {
			LanguageKind::PlutusV1 => {
				cardano_serialization_lib::PlutusScript::new(self.bytes.clone())
			},
			LanguageKind::PlutusV2 => {
				cardano_serialization_lib::PlutusScript::new_v2(self.bytes.clone())
			},
			LanguageKind::PlutusV3 => {
				cardano_serialization_lib::PlutusScript::new_v3(self.bytes.clone())
			},
		}
	}
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;
	use hex_literal::hex;
	use sidechain_domain::{McTxHash, UtxoId, UtxoIndex};

	pub(crate) const TEST_GENESIS_UTXO: UtxoId =
		UtxoId { tx_hash: McTxHash([0u8; 32]), index: UtxoIndex(0) };

	// Taken from smart-contracts repository
	pub(crate) const CANDIDATES_SCRIPT_RAW: [u8; 318] = hex!("59013b590138010000323322323322323232322222533553353232323233012225335001100f2215333573466e3c014dd7001080909802000980798051bac330033530040022200148040dd7198011a980180311000a4010660026a600400644002900019112999ab9a33710002900009805a4810350543600133003001002300f22253350011300b49103505437002215333573466e1d20000041002133005337020089001000919199109198008018011aab9d001300735573c0026ea80044028402440204c01d2401035054350030092233335573e0024016466a0146ae84008c00cd5d100124c6010446666aae7c00480288cd4024d5d080118019aba20024988c98cd5ce00080109000891001091000980191299a800880211099a80280118020008910010910911980080200191918008009119801980100100081");

	/// We know it is correct, because we are able to get the same hash as using code from smart-contract repository
	pub(crate) const CANDIDATES_SCRIPT_WITH_APPLIED_PARAMS: [u8; 362] = hex!("5901670100003323322323322323232322222533553353232323233012225335001100f2215333573466e3c014dd7001080909802000980798051bac330033530040022200148040dd7198011a980180311000a4010660026a600400644002900019112999ab9a33710002900009805a490350543600133003001002300f22253350011300b49103505437002215333573466e1d20000041002133005337020089001000919199109198008018011aab9d001300735573c0026ea80044028402440204c01d2401035054350030092233335573e0024016466a0146ae84008c00cd5d100124c6010446666aae7c00480288cd4024d5d080118019aba20024988c98cd5ce00080109000891001091000980191299a800880211099a802801180200089100109109119800802001919180080091198019801001000a6012bd8799fd8799f58200000000000000000000000000000000000000000000000000000000000000000ff00ff0001");

	#[test]
	fn apply_parameters_to_deregister() {
		let applied =
			PlutusScript::from_wrapped_cbor(&CANDIDATES_SCRIPT_RAW, Language::new_plutus_v2())
				.unwrap()
				.apply_data(TEST_GENESIS_UTXO)
				.unwrap();
		assert_eq!(hex::encode(applied.bytes), hex::encode(CANDIDATES_SCRIPT_WITH_APPLIED_PARAMS));
	}
}
