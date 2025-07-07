#[cfg(test)]
mod tests;

use crate::CmdRun;
use crate::cardano_key::{
	get_mc_payment_signing_key_from_file, get_stake_pool_verification_key_from_file,
};
use crate::cmd_traits::Deregister;
use crate::config::config_fields::{
	CARDANO_COLD_VERIFICATION_KEY_FILE, CARDANO_PAYMENT_SIGNING_KEY_FILE,
};
use crate::io::IOContext;
use crate::ogmios::config::establish_ogmios_configuration;
use anyhow::anyhow;
use ogmios_client::query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId};
use ogmios_client::query_network::QueryNetwork;
use ogmios_client::transactions::Transactions;
use partner_chains_cardano_offchain::await_tx::FixedDelayRetries;
use partner_chains_cardano_offchain::cardano_keys::CardanoPaymentSigningKey;
use partner_chains_cardano_offchain::register::run_deregister;
use sidechain_domain::{McTxHash, StakePoolPublicKey, UtxoId};

#[derive(Clone, Debug, clap::Parser)]
pub struct DeregisterCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
}

impl CmdRun for DeregisterCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		let chain_config = read_chain_config_file(context)?;
		context.print(
		  &format!("This wizard will remove the specified candidate from the committee candidates based on the following chain parameters:\n{}.\nCommittee Candidate Validator Address is '{}'\n",
          &chain_config.chain_parameters,
          &chain_config.cardano_addresses.committee_candidates_address)
    	);
		context.print("Payment signing key and cold verification key used for registration are required to deregister.");
		let payment_signing_key_path =
			CARDANO_PAYMENT_SIGNING_KEY_FILE.prompt_with_default_from_file_and_save(context);
		let payment_signing_key =
			get_mc_payment_signing_key_from_file(&payment_signing_key_path, context)?;
		let cold_vkey_path =
			CARDANO_COLD_VERIFICATION_KEY_FILE.prompt_with_default_from_file_and_save(context);
		let stake_ownership_pub_key =
			get_stake_pool_verification_key_from_file(&cold_vkey_path, context)?;
		let ogmios_config = establish_ogmios_configuration(context)?;
		let offchain = context.offchain_impl(&ogmios_config)?;

		let runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
		runtime
			.block_on(offchain.deregister(
				self.common_arguments.retries(),
				chain_config.chain_parameters.genesis_utxo,
				&payment_signing_key,
				stake_ownership_pub_key,
			))
			.map_err(|e| anyhow::anyhow!("Candidate deregistration failed: {e:?}!"))?;

		Ok(())
	}
}

fn read_chain_config_file<C: IOContext>(
	context: &C,
) -> Result<crate::config::ChainConfig, anyhow::Error> {
	let chain_config = crate::config::load_chain_config(context);
	chain_config
		.map_err(|_|anyhow!(
			"Couldn't parse chain configuration file {}. The chain configuration file that was used for registration is required in the working directory.",
			context.chain_config_file_path()))
}

impl<T> Deregister for T
where
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
{
	async fn deregister(
		&self,
		await_tx: FixedDelayRetries,
		genesis_utxo: UtxoId,
		payment_signing_key: &CardanoPaymentSigningKey,
		stake_ownership_pub_key: StakePoolPublicKey,
	) -> Result<Option<McTxHash>, String> {
		run_deregister(genesis_utxo, payment_signing_key, stake_ownership_pub_key, self, await_tx)
			.await
			.map_err(|e| e.to_string())
	}
}
