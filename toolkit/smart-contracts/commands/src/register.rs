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
		alias = "sidechain-public-keys",
		value_parser = parse_partnerchain_public_keys
	)]
	/// Candidate public keys in format PARTNER_CHAINS_KEY_HEX:AURA_KEY_HEX:GRANDPA_KEY_HEX or PARTNER_CHAINS_KEY_HEX,KEY_ID_1:KEY_1_HEX,...,KEY_ID_N:KEY_N_HEX
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
			keys: self.partner_chain_public_keys.keys,
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
