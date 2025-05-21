use crate::{
	GenesisUtxo, PaymentFilePath, option_to_json, parse_partnerchain_public_keys,
	transaction_submitted_json,
};
use partner_chains_cardano_offchain::register::{run_deregister, run_register};
use sidechain_domain::{
	AdaBasedStaking, CandidateRegistration, MainchainSignature, PermissionedCandidateData,
	SidechainSignature, StakePoolPublicKey, UtxoId,
};

/// Command for registering a candidate on the main chain
#[derive(Clone, Debug, clap::Parser)]
pub struct RegisterCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
	#[arg(long)]
	/// UTXO that will be spend when executing registration transaction, part of the registration message
	registration_utxo: UtxoId,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[arg(
		long,
		value_name = "PARTNERCHAIN_KEY:AURA_KEY:GRANDPA_KEY",
		alias = "sidechain-public-keys",
		value_parser = parse_partnerchain_public_keys
	)]
	/// Colon separated hex strings representing bytes of the Sidechain, Aura and Grandpa public keys
	partner_chain_public_keys: PermissionedCandidateData,
	#[arg(long, alias = "sidechain-signature")]
	/// Hex string of bytes of the registration message signature by partner-chain key, obtained by 'registration-signatures' command
	partner_chain_signature: SidechainSignature,
	#[arg(long)]
	/// Hex string representing bytes of the Stake Pool Verification Key
	spo_public_key: StakePoolPublicKey,
	#[arg(long)]
	/// Hex string of bytes of the registration message signature by main chain key, obtained by 'registration-signatures' command
	spo_signature: MainchainSignature,
}

impl RegisterCmd {
	/// Registers a candidate on the main chain.
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let candidate_registration = CandidateRegistration {
			stake_ownership: AdaBasedStaking {
				pub_key: self.spo_public_key,
				signature: self.spo_signature,
			},
			partner_chain_pub_key: self.partner_chain_public_keys.sidechain_public_key,
			partner_chain_signature: self.partner_chain_signature,
			own_pkh: payment_key.to_pub_key_hash(),
			registration_utxo: self.registration_utxo,
			aura_pub_key: self.partner_chain_public_keys.aura_public_key,
			grandpa_pub_key: self.partner_chain_public_keys.grandpa_public_key,
		};

		let result = run_register(
			self.genesis_utxo.into(),
			&candidate_registration,
			&payment_key,
			&client,
			self.common_arguments.retries(),
		)
		.await?;
		Ok(option_to_json(result.map(transaction_submitted_json)))
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for deregistering a candidate on the main chain
pub struct DeregisterCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[arg(long)]
	/// Hex string representing bytes of the Stake Pool Verification Key
	spo_public_key: StakePoolPublicKey,
}

impl DeregisterCmd {
	/// Deregisters a candidate on the main chain.
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_signing_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;

		let result = run_deregister(
			self.genesis_utxo.into(),
			&payment_signing_key,
			self.spo_public_key,
			&client,
			self.common_arguments.retries(),
		)
		.await?;
		Ok(option_to_json(result.map(transaction_submitted_json)))
	}
}
