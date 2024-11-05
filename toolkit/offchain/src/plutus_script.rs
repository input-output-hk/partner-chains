use anyhow::{anyhow, Context};
use cardano_serialization_lib::{Address, LanguageKind, NetworkIdKind, PlutusData};
use plutus::ToDatum;
use sidechain_domain::{MainchainAddressHash, PolicyId};
use uplc::ast::{DeBruijn, Program};

use crate::{csl::*, untyped_plutus::*};

/// Wraps a Plutus script cbor
pub struct PlutusScript {
	pub bytes: Vec<u8>,
	pub language: LanguageKind,
}

impl PlutusScript {
	pub fn from_cbor(cbor: &[u8], language: LanguageKind) -> Self {
		Self { bytes: cbor.into(), language }
	}
	pub fn from_wrapped_cbor(cbor: &[u8], language: LanguageKind) -> anyhow::Result<Self> {
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
		Ok(Self { bytes: bytes.into(), ..self })
	}

	/// Builds an CSL `Address` for plutus script from the data obtained from smart contracts.
	pub fn plutus_address(&self, network: NetworkIdKind) -> Address {
		plutus_script_address(&self.bytes, network, self.language)
	}

	// Returns PlutusData representation of the given script. It is done in the same way as on-chain code expects.
	// First, the Address is created, then it is converted to PlutusData.
	pub fn plutus_address_data(&self, network: NetworkIdKind) -> anyhow::Result<uplc::PlutusData> {
		csl_plutus_data_to_uplc(&PlutusData::from_address(&self.plutus_address(network))?)
	}

	/// Returns bech32 address of the given PlutusV2 script
	pub fn plutus_address_bech32(&self, network: NetworkIdKind) -> anyhow::Result<String> {
		self.plutus_address(network)
			.to_bech32(None)
			.context("Converting script address to bech32")
	}

	pub fn script_plutus_hash(&self) -> [u8; 28] {
		plutus_script_hash(&self.bytes, self.language)
	}

	pub fn script_plutus_address(&self) -> MainchainAddressHash {
		MainchainAddressHash(self.script_plutus_hash())
	}

	pub fn plutus_policy_id(&self) -> PolicyId {
		PolicyId(self.script_plutus_hash())
	}
}
