use crate::csl::*;
use anyhow::{anyhow, Context, Error};
use cardano_serialization_lib::{
	Address, JsError, Language, LanguageKind, NetworkIdKind, PlutusData, ScriptHash,
};
use ogmios_client::types::OgmiosScript;
use plutus::ToDatum;
use sidechain_domain::{AssetId, AssetName, PolicyId};
use uplc::{
	ast::{DeBruijn, Program},
	plutus_data,
};

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
		ogmios_script.try_into()
	}

	/// This function is needed to create [PlutusScript] from scripts in [raw_scripts],
	/// which are encoded as a cbor byte string containing the cbor of the script
	/// itself. This function removes this layer of wrapping.
	pub fn from_wrapped_cbor(
		plutus_script_raw_cbor: &[u8],
		language: Language,
	) -> anyhow::Result<Self> {
		let plutus_script_bytes: uplc::PlutusData = minicbor::decode(plutus_script_raw_cbor)?;
		let plutus_script_bytes = match plutus_script_bytes {
			uplc::PlutusData::BoundedBytes(bb) => Ok(bb),
			_ => Err(anyhow!("expected validator raw to be BoundedBytes")),
		}?;
		Ok(Self::from_cbor(&plutus_script_bytes, language))
	}

	pub fn apply_data(self, data: impl ToDatum) -> Result<Self, anyhow::Error> {
		let data = plutus_data(&minicbor::to_vec(data.to_datum()).expect("to_vec is Infallible"))
			.expect("trasformation from PC Datum to pallas PlutusData can't fail");
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

	pub fn unapply_data_uplc(&self) -> Result<uplc::PlutusData, anyhow::Error> {
		let mut buffer = Vec::new();
		let program = Program::<DeBruijn>::from_cbor(&self.bytes, &mut buffer).unwrap();
		match program.term {
			uplc::ast::Term::Apply { function: _, argument } => {
				let res: Result<uplc::PlutusData, String> = (*argument).clone().try_into();
				res.map_err(|e| anyhow!(e))
			},
			_ => Err(anyhow!("Given Plutus Script is not an applied term")),
		}
	}

	pub fn unapply_data_csl(&self) -> Result<PlutusData, anyhow::Error> {
		let uplc_pd = self.unapply_data_uplc()?;
		let cbor_bytes = minicbor::to_vec(uplc_pd).expect("to_vec has Infallible error type");
		Ok(PlutusData::from_bytes(cbor_bytes).expect("UPLC encoded PlutusData is valid"))
	}

	/// Builds an CSL `Address` for plutus script from the data obtained from smart contracts.
	pub fn address(&self, network: NetworkIdKind) -> Address {
		script_address(&self.bytes, network, self.language)
	}

	// Returns PlutusData representation of the given script. It is done in the same way as on-chain code expects.
	// First, the Address is created, then it is converted to PlutusData.
	pub fn address_data(&self, network: NetworkIdKind) -> anyhow::Result<uplc::PlutusData> {
		let mut se = cbor_event::se::Serializer::new_vec();
		cbor_event::se::Serialize::serialize(
			&PlutusData::from_address(&self.address(network))?,
			&mut se,
		)
		.map_err(|e| anyhow!(e))?;
		let bytes = se.finalize();
		minicbor::decode(&bytes).map_err(|e| anyhow!(e.to_string()))
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

	pub fn empty_name_asset(&self) -> AssetId {
		AssetId { policy_id: self.policy_id(), asset_name: AssetName::empty() }
	}

	pub fn asset(
		&self,
		asset_name: cardano_serialization_lib::AssetName,
	) -> Result<AssetId, JsError> {
		Ok(AssetId { policy_id: self.policy_id(), asset_name: AssetName::from_csl(asset_name)? })
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

impl TryFrom<ogmios_client::types::OgmiosScript> for PlutusScript {
	type Error = Error;

	fn try_from(script: ogmios_client::types::OgmiosScript) -> Result<Self, Self::Error> {
		let language = match script.language.as_str() {
			"plutus:v1" => Language::new_plutus_v1(),
			"plutus:v2" => Language::new_plutus_v2(),
			"plutus:v3" => Language::new_plutus_v3(),
			_ => return Err(anyhow!("Unsupported Plutus language version: {}", script.language)),
		};
		Ok(Self::from_cbor(&script.cbor, language))
	}
}

impl From<PlutusScript> for ogmios_client::types::OgmiosScript {
	fn from(val: PlutusScript) -> Self {
		ogmios_client::types::OgmiosScript {
			language: match val.language.kind() {
				LanguageKind::PlutusV1 => "plutus:v1",
				LanguageKind::PlutusV2 => "plutus:v2",
				LanguageKind::PlutusV3 => "plutus:v3",
			}
			.to_string(),
			cbor: val.bytes,
			json: None,
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

	#[test]
	fn unapply_term_csl() {
		let applied =
			PlutusScript::from_wrapped_cbor(&CANDIDATES_SCRIPT_RAW, Language::new_plutus_v2())
				.unwrap()
				.apply_data(TEST_GENESIS_UTXO)
				.unwrap();
		let data: PlutusData = applied.unapply_data_csl().unwrap();
		assert_eq!(
			data,
			PlutusData::from_bytes(minicbor::to_vec(TEST_GENESIS_UTXO.to_datum()).unwrap())
				.unwrap()
		)
	}

	#[test]
	fn unapply_term_uplc() {
		let applied =
			PlutusScript::from_wrapped_cbor(&CANDIDATES_SCRIPT_RAW, Language::new_plutus_v2())
				.unwrap()
				.apply_data(TEST_GENESIS_UTXO)
				.unwrap();
		let data: uplc::PlutusData = applied.unapply_data_uplc().unwrap();
		assert_eq!(
			data,
			plutus_data(&minicbor::to_vec(TEST_GENESIS_UTXO.to_datum()).unwrap()).unwrap()
		)
	}
}
