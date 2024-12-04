use log4rs::{
	append::{console::ConsoleAppender, file::FileAppender},
	config::Appender,
};
use sidechain_domain::MainchainPrivateKey;

pub mod get_scripts;
pub mod init_governance;

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum SmartContractsCmd {
	/// Print validator addresses and policy IDs of Partner Chain smart contracts
	GetScripts(get_scripts::GetScripts),
	/// Initialize Partner Chain governance
	InitGovernance(init_governance::InitGovernanceCmd),
}

#[derive(Clone, Debug, clap::Parser)]
#[command(author, version, about, long_about = None)]
pub struct CommonArguments {
	#[arg(default_value = "http://localhost:1337", long, short = 'O')]
	ogmios_host: String,
}

type CmdResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

impl SmartContractsCmd {
	pub async fn execute(self) -> CmdResult<()> {
		match self {
			Self::InitGovernance(cmd) => cmd.execute().await,
			Self::GetScripts(cmd) => cmd.execute().await,
		}
	}

	pub fn execute_blocking(self) -> CmdResult<()> {
		setup_logging()?;

		tokio::runtime::Runtime::new()?.block_on(self.execute())
	}
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CardanoKeyFileContent {
	cbor_hex: String,
}

pub(crate) fn read_private_key_from_file(path: &str) -> CmdResult<MainchainPrivateKey> {
	let file_content_str = String::from_utf8(std::fs::read(path)?)?;
	let file_content = serde_json::from_str::<CardanoKeyFileContent>(&file_content_str)?;
	let key_hex = (file_content.cbor_hex.strip_prefix("5820"))
		.ok_or("CBOR prefix missing in payment key".to_string())?;
	let key_bytes = (hex::decode(key_hex)?.try_into())
		.map_err(|_| format!("{} is not the valid lengh of 32", key_hex))?;
	Ok(MainchainPrivateKey(key_bytes))
}

pub fn setup_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let stdout = ConsoleAppender::builder().build();
	let ogmios_log = FileAppender::builder().build("ogmios_client.log")?;

	let log_config = log4rs::config::Config::builder()
		.appender(Appender::builder().build("stdout", Box::new(stdout)))
		.appender(Appender::builder().build("ogmios-log", Box::new(ogmios_log)))
		.logger(
			log4rs::config::Logger::builder()
				.appender("ogmios-log")
				.additive(false)
				.build("ogmios_client::jsonrpsee", log::LevelFilter::Debug),
		)
		.build(log4rs::config::Root::builder().appender("stdout").build(log::LevelFilter::Info))?;

	log4rs::init_config(log_config)?;

	Ok(())
}
