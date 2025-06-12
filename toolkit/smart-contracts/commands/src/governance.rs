use crate::{GenesisUtxo, PaymentFilePath, option_to_json};
use partner_chains_cardano_offchain::{
	governance::{MultiSigParameters, get_governance_policy_summary},
	init_governance::run_init_governance,
	update_governance::run_update_governance,
};
use sidechain_domain::{MainchainKeyHash, UtxoId};

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
/// Commands for managing the governance of the partner chain
pub enum GovernanceCmd {
	/// Initialize Partner Chain governance
	Init(InitGovernanceCmd),
	/// Update Partner Chain governance
	Update(UpdateGovernanceCmd),
	/// Prints JSON summary of the current governance policy of a chain. Prints null if governance policy has not been set for given genesis utxo.
	GetPolicy(GetPolicyCmd),
}

impl GovernanceCmd {
	/// Executes the internal command
	pub async fn execute(self) -> crate::SubCmdResult {
		match self {
			Self::Init(cmd) => cmd.execute().await,
			Self::Update(cmd) => cmd.execute().await,
			Self::GetPolicy(cmd) => cmd.execute().await,
		}
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for initializing the governance of the partner chain
pub struct InitGovernanceCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(short = 'g', long, num_args = 1.., value_delimiter = ' ')]
	/// Governance authority to be set, list of hex encoded, space delimited public key hashes, order does not matter
	governance_authority: Vec<MainchainKeyHash>,
	#[arg(short = 't', long)]
	/// Minimum number of authorities required to sign a transaction
	threshold: u8,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	/// Genesis UTXO of the new chain, it will be spent durning initialization. If not set, then one will be selected from UTXOs of the payment key.
	#[arg(long, short = 'c')]
	genesis_utxo: Option<UtxoId>,
}

impl InitGovernanceCmd {
	/// Initializes the governance of the partner chain.
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
			self.common_arguments.retries(),
		)
		.await?;
		Ok(serde_json::json!(result))
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for updating the governance of the partner chain
pub struct UpdateGovernanceCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	/// New governance authorities keys hashes, hex encoded, space delimited, order does not matter
	#[arg(short = 'g', long, num_args = 1.., value_delimiter = ' ')]
	governance_authority: Vec<MainchainKeyHash>,
	/// Minimum number of authorities required to sign a transaction
	#[arg(short = 't', long)]
	threshold: u8,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO. This has to be the same as the one used during governance initialization.
	genesis_utxo: GenesisUtxo,
}

impl UpdateGovernanceCmd {
	/// Updates the governance of the partner chain.
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;

		let multisig_parameters =
			MultiSigParameters::new(&self.governance_authority, self.threshold)?;

		let result = run_update_governance(
			&multisig_parameters,
			&payment_key,
			self.genesis_utxo.into(),
			&client,
			self.common_arguments.retries(),
		)
		.await?;
		Ok(serde_json::json!(result))
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for getting the current governance authorities and threshold.
pub struct GetPolicyCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long, short = 'c')]
	/// Genesis UTXO that identifies the partner chain.
	genesis_utxo: UtxoId,
}

impl GetPolicyCmd {
	/// Gets the current governance authorities and threshold.
	pub async fn execute(self) -> crate::SubCmdResult {
		let client = self.common_arguments.get_ogmios_client().await?;
		let summary = get_governance_policy_summary(self.genesis_utxo, &client).await?;
		Ok(option_to_json(summary))
	}
}
