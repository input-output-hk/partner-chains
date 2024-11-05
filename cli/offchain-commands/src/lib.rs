#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum OffchainCmd {
	/// Get the script addresses for a given sidechain
	Addresses,
	/// Register a committee candidate
	Register,
	/// Deregister a committee candidate
	Deregister,
	/// Insert or update D parameter
	UpsertDParameter,
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
pub struct CommonArguments<SidechainParams: clap::Args> {
	#[clap(flatten)]
	sidechain_params: SidechainParams,
	#[arg(default_value = "localhost")]
	ogmios_host: String,
	#[arg(default_value = "1337")]
	ogmios_port: u32,
	#[arg(default_value = "true")]
	ogmios_secure: bool,
}

impl OffchainCmd {
	pub fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		match self {
			_ => Err(format!("Command {self:?} is not yet implemented").into()),
		}
	}
}
