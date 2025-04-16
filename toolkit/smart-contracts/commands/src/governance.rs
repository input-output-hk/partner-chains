use crate::{option_to_json, PaymentFilePath};
use partner_chains_cardano_offchain::{
	await_tx::FixedDelayRetries,
	governance::{get_governance_policy_summary, MultiSigParameters},
	init_governance::run_init_governance,
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
	/// Prints JSON summary of the current governance policy of a chain. Prints null if governance policy has not been set for given genesis utxo.
	GetPolicy(GetPolicyCmd),
}

impl GovernanceCmd {
	pub async fn execute(self) -> crate::SubCmdResult {
		match self {
			Self::Init(cmd) => cmd.execute().await,
			Self::Update(cmd) => cmd.execute().await,
			Self::GetPolicy(cmd) => cmd.execute().await,
		}
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct InitGovernanceCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	/// Governance authority to be set, hex encoded, space delimited, order does not matter
	#[arg(short = 'g', long, num_args = 1.., value_delimiter = ' ')]
	governance_authority: Vec<MainchainKeyHash>,
	/// Governance threshold to be set
	#[arg(short = 't', long)]
	threshold: u8,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	/// Genesis UTXO of the new chain, it will be spent durning initialization. If not set, then one will be selected from UTXOs of the payment key.
	#[arg(long, short = 'c')]
	genesis_utxo: Option<UtxoId>,
}

impl InitGovernanceCmd {
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;

		let multisig_parameters =
			MultiSigParameters::new(&self.governance_authority, self.threshold)?;

		let result = run_init_governance(
			&multisig_parameters,
			&payment_key,
			self.genesis_utxo,
			&client,
			FixedDelayRetries::five_minutes(),
		)
		.await?;
		Ok(serde_json::json!(result))
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct UpdateGovernanceCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	/// New governance authorities keys hashes, hex encoded, space delimited, order does not matter
	#[arg(short = 'g', long, num_args = 1.., value_delimiter = ' ')]
	governance_authority: Vec<MainchainKeyHash>,
	/// Governance threshold to be set
	#[arg(short = 't', long)]
	threshold: u8,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	/// Genesis UTXO of the chain
	#[arg(long, short = 'c')]
	genesis_utxo: UtxoId,
}

impl UpdateGovernanceCmd {
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;

		let multisig_parameters =
			MultiSigParameters::new(&self.governance_authority, self.threshold)?;

		let result = run_update_governance(
			&multisig_parameters,
			&payment_key,
			self.genesis_utxo,
			&client,
			FixedDelayRetries::five_minutes(),
		)
		.await?;
		Ok(serde_json::json!(result))
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct GetPolicyCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	/// Genesis UTXO that identifies the partner chain.
	#[arg(long, short = 'c')]
	genesis_utxo: UtxoId,
}

impl GetPolicyCmd {
	pub async fn execute(self) -> crate::SubCmdResult {
		let client = self.common_arguments.get_ogmios_client().await?;
		let summary = get_governance_policy_summary(self.genesis_utxo, &client).await?;
		Ok(option_to_json(summary))
	}
}
