use jsonrpsee::http_client::HttpClient;
use partner_chains_cardano_offchain::{await_tx::FixedDelayRetries, register::run_register};
use sidechain_domain::{
	AdaBasedStaking, AuraPublicKey, CandidateRegistration, GrandpaPublicKey, MainchainPublicKey,
	MainchainSignature, SidechainPublicKey, SidechainSignature, UtxoId,
};

use crate::{parse_sidechain_public_keys, read_private_key_from_file};

#[derive(Clone, Debug, clap::Parser)]
pub struct RegisterCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	genesis_utxo: UtxoId,
	#[arg(long)]
	registration_utxo: UtxoId,
	#[arg(long)]
	payment_key_file: String,
	#[arg(long, value_name = "SIDECHAIN_KEY:AURA_KEY:GRANDPA_KEY", value_parser = parse_sidechain_public_keys)]
	sidechain_public_keys: (SidechainPublicKey, AuraPublicKey, GrandpaPublicKey),
	#[arg(long)]
	sidechain_signature: SidechainSignature,
	#[arg(long)]
	spo_public_key: MainchainPublicKey,
	#[arg(long)]
	spo_signature: MainchainSignature,
}

impl RegisterCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = read_private_key_from_file(&self.payment_key_file)?;
		let client = HttpClient::builder().build(self.common_arguments.ogmios_host)?;
		let candidate_registration = CandidateRegistration {
			stake_ownership: AdaBasedStaking {
				pub_key: self.spo_public_key,
				signature: self.spo_signature,
			},
			sidechain_pub_key: self.sidechain_public_keys.0,
			sidechain_signature: self.sidechain_signature,
			own_pkh: crate::payment_signing_key_to_mainchain_address_hash(payment_key.clone())?,
			registration_utxo: self.registration_utxo,
			aura_pub_key: self.sidechain_public_keys.1,
			grandpa_pub_key: self.sidechain_public_keys.2,
		};

		run_register(
			self.genesis_utxo,
			&candidate_registration,
			payment_key,
			&client,
			FixedDelayRetries::two_minutes(),
		)
		.await?;

		Ok(())
	}
}
