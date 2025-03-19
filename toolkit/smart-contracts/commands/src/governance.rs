use crate::PaymentFilePath;
use anyhow::anyhow;
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
	/// Governance authority to be set
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
		let client = self.common_arguments.get_ogmios_client().await?;

		let result = run_init_governance(
			self.governance_authority,
			&payment_key,
			self.genesis_utxo,
			&client,
			FixedDelayRetries::two_minutes(),
		)
		.await?;
		println!("{}", serde_json::to_string_pretty(&result)?);
		Ok(())
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct UpdateGovernanceCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	/// New governance authorities keys hashes, hex encoded, space delimited, order does not matter
	#[arg(short = 'g', long, num_args = 1.., value_delimiter = ' ')]
	new_governance_authority: Vec<MainchainKeyHash>,
	/// Governance threshold to be set
	#[arg(long)]
	new_governance_threshold: u8,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	/// Genesis UTXO of the chain
	#[arg(long, short = 'c')]
	genesis_utxo: UtxoId,
}

impl UpdateGovernanceCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		if self.new_governance_threshold > self.new_governance_authority.len() as u8 {
			return Err(anyhow!(
				"New governance threshold is greater than the number of governance authorities"
			)
			.into());
		}

		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;

		let result = run_update_governance(
			&self.new_governance_authority,
			self.new_governance_threshold,
			&payment_key,
			self.genesis_utxo,
			&client,
			FixedDelayRetries::two_minutes(),
		)
		.await?;
		println!("{}", serde_json::to_value(result)?);
		Ok(())
	}
}
