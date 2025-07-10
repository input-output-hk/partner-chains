use crate::csl::*;
use anyhow::{Context, Error, anyhow};
use cardano_serialization_lib::{
	Address, JsError, Language, LanguageKind, NetworkIdKind, PlutusData, ScriptHash,
};
use plutus::ToDatum;
use sidechain_domain::{AssetId, AssetName, PolicyId};
use std::marker::PhantomData;
use uplc::ast::{DeBruijn, Program};

/// Wraps a Plutus script CBOR
#[derive(Clone, PartialEq, Eq)]
pub struct PlutusScript {
	/// CBOR bytes of the encoded Plutus script
	pub bytes: Vec<u8>,
	/// The language of the encoded Plutus script
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
	/// Constructs a [PlutusScript].
	pub fn from_cbor(cbor: &[u8], language: Language) -> Self {
		Self { bytes: cbor.into(), language }
	}

	/// Constructs a V2 [PlutusScript].
	pub fn v2_from_cbor(plutus_script_bytes: &[u8]) -> anyhow::Result<Self> {
		Ok(Self::from_cbor(&plutus_script_bytes, Language::new_plutus_v2()))
	}

	/// Applies the [PlutusScript] to the [uplc::PlutusData], binding it to its first argument.
	/// For example, if the [PlutusScript] has signature:
	///   `script :: A -> B -> C`
	/// After application it will be:
	///   `script' :: B -> C`
	pub fn apply_data_uplc(self, data: uplc::PlutusData) -> Result<Self, anyhow::Error> {
		let mut buffer = Vec::new();
		let mut program = Program::<DeBruijn>::from_cbor(&self.bytes, &mut buffer)
			.map_err(|e| anyhow!(e.to_string()))?;
		program = program.apply_data(data);
		let bytes = program
			.to_cbor()
			.map_err(|_| anyhow!("Couldn't encode resulting script as CBOR."))?;
		Ok(Self { bytes, ..self })
	}

	/// Extracts the last applied argument from a [PlutusScript].
	/// For example, if a [PlutusScript] `script` has been applied to [uplc::PlutusData] `data`:
	/// `script' = script data`, then [Self::unapply_data_uplc] called on `script'` will return `data`.
	pub fn unapply_data_uplc(&self) -> anyhow::Result<uplc::PlutusData> {
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

	/// Extracts the last applied argument from a [PlutusScript], and returns it as CSL [PlutusData].
	/// For more details see [Self::unapply_data_uplc].
	pub fn unapply_data_csl(&self) -> Result<PlutusData, anyhow::Error> {
		let uplc_pd = self.unapply_data_uplc()?;
		let cbor_bytes = minicbor::to_vec(uplc_pd).expect("to_vec has Infallible error type");
		Ok(PlutusData::from_bytes(cbor_bytes).expect("UPLC encoded PlutusData is valid"))
	}

	/// Builds an CSL [Address] for plutus script from the data obtained from smart contracts.
	pub fn address(&self, network: NetworkIdKind) -> Address {
		script_address(&self.bytes, network, self.language)
	}

	/// Returns [uplc::PlutusData] representation of the given script. It is done in the same way as on-chain code expects.
	/// First, the [Address] is created, then it is converted to [uplc::PlutusData].
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

	/// Returns script hash of [PlutusScript] as array of bytes.
	pub fn script_hash(&self) -> [u8; 28] {
		plutus_script_hash(&self.bytes, self.language)
	}

	/// Returns [ScriptHash] of [PlutusScript].
	pub fn csl_script_hash(&self) -> ScriptHash {
		ScriptHash::from(self.script_hash())
	}

	/// Returns [PolicyId] of [PlutusScript].
	pub fn policy_id(&self) -> PolicyId {
		PolicyId(self.script_hash())
	}

	/// Returns [AssetId] of [PlutusScript].
	pub fn empty_name_asset(&self) -> AssetId {
		AssetId { policy_id: self.policy_id(), asset_name: AssetName::empty() }
	}

	/// Constructs [AssetId] with given `asset_name` and taking the [PlutusScript] as a minting policy.
	pub fn asset(
		&self,
		asset_name: cardano_serialization_lib::AssetName,
	) -> Result<AssetId, JsError> {
		Ok(AssetId { policy_id: self.policy_id(), asset_name: AssetName::from_csl(asset_name)? })
	}

	/// Converts [PlutusScript] to CSL [cardano_serialization_lib::PlutusScript].
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

impl From<raw_scripts::RawScript> for PlutusScript {
	fn from(value: raw_scripts::RawScript) -> Self {
		PlutusScript::v2_from_cbor(value.0).expect("raw_scripts provides valid scripts")
	}
}

/// Applies arguments to a Plutus script.
/// The first argument is the script, the rest of the arguments are the datums that will be applied.
/// * The script can be any type that implements [`Into<PlutusScript>`] for example [raw_scripts::RawScript].
/// * The arguments can be any type that implements [`Into<PlutusDataWrapper>`]. Implementations are provided for
///   [uplc::PlutusData] and [plutus::Datum].
/// Returns [anyhow::Result<uplc::PlutusData>].
///
/// Example:
/// ```rust,ignore
/// plutus_script![SOME_SCRIPT, genesis_utxo, plutus::Datum::ListDatum(Vec::new())]
/// ```
#[macro_export]
macro_rules! plutus_script {
    ($ps:expr $(,$args:expr)*) => (
		{
			let script = $crate::plutus_script::PlutusScript::from($ps);
			plutus_script!(@inner, script $(,$args)*)
		}
	);
	(@inner, $ps:expr) => (Ok::<crate::plutus_script::PlutusScript, anyhow::Error>($ps));
    (@inner, $ps:expr, $arg:expr $(,$args:expr)*) => (
		$ps.apply_data_uplc($crate::plutus_script::PlutusDataWrapper::from($arg).0)
	    	.and_then(|ps| plutus_script!(@inner, ps $(,$args)*))
    )
}

/// Wrapper type for [uplc::PlutusData].
///
/// Note: The type argument is needed to make the compiler accept the implementation for
/// `impl<T: ToDatum> From<T> for PlutusDataWrapper<T>`.
pub struct PlutusDataWrapper<T>(pub uplc::PlutusData, PhantomData<T>);

impl<T> PlutusDataWrapper<T> {
	/// Constructs [PlutusDataWrapper].
	pub fn new(d: uplc::PlutusData) -> Self {
		Self(d, PhantomData)
	}
}

impl From<uplc::PlutusData> for PlutusDataWrapper<()> {
	fn from(value: uplc::PlutusData) -> Self {
		PlutusDataWrapper::new(value)
	}
}

impl<T: ToDatum> From<T> for PlutusDataWrapper<T> {
	fn from(value: T) -> Self {
		PlutusDataWrapper::new(to_plutus_data(value.to_datum()))
	}
}

impl From<raw_scripts::ScriptId> for PlutusDataWrapper<()> {
	fn from(value: raw_scripts::ScriptId) -> Self {
		PlutusDataWrapper::new(to_plutus_data((value as u32).to_datum()))
	}
}

fn to_plutus_data(datum: plutus::Datum) -> uplc::PlutusData {
	uplc::plutus_data(&minicbor::to_vec(datum).expect("to_vec is Infallible"))
		.expect("transformation from PC Datum to pallas PlutusData can't fail")
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;
	use hex_literal::hex;
	use raw_scripts::RawScript;
	use sidechain_domain::{McTxHash, UtxoId, UtxoIndex};

	pub(crate) const TEST_GENESIS_UTXO: UtxoId =
		UtxoId { tx_hash: McTxHash([0u8; 32]), index: UtxoIndex(0) };

	// Taken from smart-contracts repository
	pub(crate) const CANDIDATES_SCRIPT_RAW: RawScript = RawScript(&hex!(
		"59013b590138010000323322323322323232322222533553353232323233012225335001100f2215333573466e3c014dd7001080909802000980798051bac330033530040022200148040dd7198011a980180311000a4010660026a600400644002900019112999ab9a33710002900009805a4810350543600133003001002300f22253350011300b49103505437002215333573466e1d20000041002133005337020089001000919199109198008018011aab9d001300735573c0026ea80044028402440204c01d2401035054350030092233335573e0024016466a0146ae84008c00cd5d100124c6010446666aae7c00480288cd4024d5d080118019aba20024988c98cd5ce00080109000891001091000980191299a800880211099a80280118020008910010910911980080200191918008009119801980100100081"
	));

	/// We know it is correct, because we are able to get the same hash as using code from smart-contract repository
	pub(crate) const CANDIDATES_SCRIPT_WITH_APPLIED_PARAMS: [u8; 362] = hex!(
		"5901670100003323322323322323232322222533553353232323233012225335001100f2215333573466e3c014dd7001080909802000980798051bac330033530040022200148040dd7198011a980180311000a4010660026a600400644002900019112999ab9a33710002900009805a490350543600133003001002300f22253350011300b49103505437002215333573466e1d20000041002133005337020089001000919199109198008018011aab9d001300735573c0026ea80044028402440204c01d2401035054350030092233335573e0024016466a0146ae84008c00cd5d100124c6010446666aae7c00480288cd4024d5d080118019aba20024988c98cd5ce00080109000891001091000980191299a800880211099a802801180200089100109109119800802001919180080091198019801001000a6012bd8799fd8799f58200000000000000000000000000000000000000000000000000000000000000000ff00ff0001"
	);

	#[test]
	fn apply_parameters_to_deregister() {
		let applied = plutus_script![CANDIDATES_SCRIPT_RAW, TEST_GENESIS_UTXO].unwrap();
		assert_eq!(hex::encode(applied.bytes), hex::encode(CANDIDATES_SCRIPT_WITH_APPLIED_PARAMS));
	}

	#[test]
	fn unapply_term_csl() {
		let applied = plutus_script![CANDIDATES_SCRIPT_RAW, TEST_GENESIS_UTXO].unwrap();
		let data: PlutusData = applied.unapply_data_csl().unwrap();
		assert_eq!(
			data,
			PlutusData::from_bytes(minicbor::to_vec(TEST_GENESIS_UTXO.to_datum()).unwrap())
				.unwrap()
		)
	}

	#[test]
	fn unapply_term_uplc() {
		let applied = plutus_script![CANDIDATES_SCRIPT_RAW, TEST_GENESIS_UTXO].unwrap();
		let data: uplc::PlutusData = applied.unapply_data_uplc().unwrap();
		assert_eq!(
			data,
			uplc::plutus_data(&minicbor::to_vec(TEST_GENESIS_UTXO.to_datum()).unwrap()).unwrap()
		)
	}
}
