use crate::key_params::MainchainSigningKeyParam;
use anyhow::Ok;
use byte_string::ByteString;
use clap::Parser;
use parity_scale_codec::Encode;
use serde::Serialize;
use serde_json::json;
use sidechain_domain::*;
use sp_address_associations::AddressAssociationSignedMessage;
use std::str::FromStr;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct AddressAssociationSignaturesCmd<
	PartnerchainAddress: Clone + Sync + Send + FromStr + 'static,
> {
	/// Genesis UTXO of the target Partner Chain
	#[arg(long)]
	pub genesis_utxo: UtxoId,
	/// Partner Chain address to be associated with the Cardano address
	#[arg(long, value_parser=parse::<PartnerchainAddress>)]
	pub partnerchain_address: PartnerchainAddress,
	/// Cardano ECDSA signing key. Its public key will be associated with partnerchain_address.
	#[arg(long)]
	pub signing_key: MainchainSigningKeyParam,
}

fn parse<T: FromStr>(s: &str) -> Result<T, String> {
	T::from_str(s).map_err(|_| "Failed to parse Partner Chain address".to_owned())
}

impl<PartnerchainAddress> AddressAssociationSignaturesCmd<PartnerchainAddress>
where
	PartnerchainAddress: Serialize + Clone + Sync + Send + FromStr + Encode + 'static,
{
	pub fn execute(&self) -> anyhow::Result<()> {
		let Self { partnerchain_address, genesis_utxo, signing_key } = self.clone();
		let msg = AddressAssociationSignedMessage {
			mainchain_vkey: signing_key.vkey(),
			partnerchain_address: partnerchain_address.clone(),
			genesis_utxo,
		};
		let encoded = msg.encode();
		let signature: ByteString = signing_key.0.sign(&encoded).s_bytes().to_vec().into();
		let output = json!({
			"pcAddr": partnerchain_address,
			"signature": signature,
			"verificationKey": signing_key.vkey()

		});
		println!("{}", serde_json::to_string_pretty(&output)?);
		Ok(())
	}
}
