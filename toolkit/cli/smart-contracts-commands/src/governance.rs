use crate::PaymentFilePath;
use ogmios_client::jsonrpsee::client_for_url;
use partner_chains_cardano_offchain::{
	await_tx::FixedDelayRetries, init_governance::run_init_governance,
	update_governance::run_update_governance,
};
use sidechain_domain::{MainchainKeyHash, UtxoId};

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceCmd {
	/// Initialize Partner Chain governance
	Init(InitGovernanceCmd),
	/// Update Partner Chain governance
	Update(UpdateGovernanceCmd),
}

impl GovernanceCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		match self {
			Self::Init(cmd) => cmd.execute().await,
			Self::Update(cmd) => cmd.execute().await,
		}
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct InitGovernanceCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	/// Governance authority hash to be set.
	#[arg(long, short = 'g')]
	governance_authority: MainchainKeyHash,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	/// Genesis UTXO of the new chain, it will be spent durning initialization. If not set, then one will be selected from UTXOs of the payment key.
	#[arg(long, short = 'c')]
	genesis_utxo: Option<UtxoId>,
}

impl InitGovernanceCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let client = client_for_url(&self.common_arguments.ogmios_url).await?;

		run_init_governance(
			self.governance_authority,
			&payment_key,
			self.genesis_utxo,
			&client,
			FixedDelayRetries::two_minutes(),
		)
		.await?;
		Ok(())
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct UpdateGovernanceCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	/// Governance authority hash to be set.
	#[arg(long, short = 'g')]
	new_governance_authority: MainchainKeyHash,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	/// Genesis UTXO of the chain
	#[arg(long, short = 'c')]
	genesis_utxo: UtxoId,
}

impl UpdateGovernanceCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let client = client_for_url(&self.common_arguments.ogmios_url).await?;

		run_update_governance(
			self.new_governance_authority,
			&payment_key,
			self.genesis_utxo,
			&client,
			FixedDelayRetries::two_minutes(),
		)
		.await?;

		Ok(())
	}
}
