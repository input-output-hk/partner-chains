use jsonrpsee::http_client::HttpClient;
use partner_chains_cardano_offchain::{
	await_tx::FixedDelayRetries,
	register::{run_deregister, run_register},
};
use sidechain_domain::{
	AdaBasedStaking, CandidateRegistration, MainchainPublicKey, MainchainSignature,
	PermissionedCandidateData, SidechainSignature, UtxoId,
};

use crate::{parse_partnerchain_public_keys, read_private_key_from_file};

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
	#[arg(
		long,
		value_name = "PARTNERCHAIN_KEY:AURA_KEY:GRANDPA_KEY",
		alias = "sidechain-public-keys",
		value_parser = parse_partnerchain_public_keys
	)]
	partnerchain_public_keys: PermissionedCandidateData,
	#[arg(long, alias = "sidechain-signature")]
	partnerchain_signature: SidechainSignature,
	#[arg(long)]
	spo_public_key: MainchainPublicKey,
	#[arg(long)]
	spo_signature: MainchainSignature,
}

impl RegisterCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = read_private_key_from_file(&self.payment_key_file)?;
		let client = HttpClient::builder().build(self.common_arguments.ogmios_url)?;
		let candidate_registration = CandidateRegistration {
			stake_ownership: AdaBasedStaking {
				pub_key: self.spo_public_key,
				signature: self.spo_signature,
			},
			partnerchain_pub_key: self.partnerchain_public_keys.sidechain_public_key,
			partnerchain_signature: self.partnerchain_signature,
			own_pkh: payment_key.to_pub_key_hash(),
			registration_utxo: self.registration_utxo,
			aura_pub_key: self.partnerchain_public_keys.aura_public_key,
			grandpa_pub_key: self.partnerchain_public_keys.grandpa_public_key,
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

#[derive(Clone, Debug, clap::Parser)]
pub struct DeregisterCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	genesis_utxo: UtxoId,
	#[arg(long)]
	payment_key_file: String,
	#[arg(long)]
	spo_public_key: MainchainPublicKey,
}

impl DeregisterCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_signing_key = read_private_key_from_file(&self.payment_key_file)?;
		let client = HttpClient::builder().build(self.common_arguments.ogmios_url)?;

		run_deregister(
			self.genesis_utxo,
			payment_signing_key,
			self.spo_public_key,
			&client,
			FixedDelayRetries::two_minutes(),
		)
		.await?;

		Ok(())
	}
}
