use sidechain_domain::*;
use std::error::Error;

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum OffchainCmd {
	/// Get the script addresses for a given sidechain
	Addresses,
	/// Register a committee candidate
	Register,
	/// Deregister a committee candidate
	Deregister,
	/// Insert new D parameter
	InsertDParameter,
	/// Update a D parameter
	UpdateDParameter,
	/// Set or update permissioned candidates list
	UpdatePermissionedCandidates,
	/// Create a new token reserve
	ReserveCreate,
	/// Deposit assets to existing reserve
	ReserveDeposit,
	/// Empty and remove an existing reserve
	ReserveHandover,
	/// Release currently available funds from an existing
	ReserveRelease,
}

#[derive(Clone, Debug, clap::Parser)]
#[command(author, version, about, long_about = None)]
pub struct SidechainParams {
	#[arg(long)]
	pub chain_id: u16,
	#[arg(long)]
	pub genesis_committee_utxo: UtxoId,
	#[arg(long)]
	pub threshold_numerator: u64,
	#[arg(long)]
	pub threshold_denominator: u64,
	#[arg(long)]
	pub governance_authority: MainchainAddressHash,
}

#[derive(Clone, Debug, clap::Parser, clap::ValueEnum)]
pub enum Network {
	Mainnet,
	Testnet,
}

#[derive(Clone, Debug, clap::Parser)]
#[command(author, version, about, long_about = None)]
pub struct CommonArguments {
	#[clap(flatten)]
	sidechain_params: SidechainParams,
	payment_signing_key_file: String,
	stake_signing_key_file: Option<String>,
	#[arg(default_value = "localhost")]
	ogmios_host: String,
	#[arg(default_value = "1337")]
	ogmios_port: u32,
	#[arg(default_value = "true")]
	ogmios_secure: bool,
	#[arg(default_value = "localhost")]
	kupo_host: String,
	#[arg(default_value = "1442")]
	kupo_port: u32,
	#[arg(default_value = "true")]
	kupo_secure: bool,
	network: Network,
}

impl OffchainCmd {
	pub fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
		match self {
			_ => Err(format!("Command {self:?} is not yet implemented").into()),
		}
	}
}
