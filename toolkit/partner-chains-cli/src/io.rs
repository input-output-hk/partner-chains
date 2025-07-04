use crate::cmd_traits::*;
use crate::config::{ConfigFile, ServiceConfig};
use crate::ogmios::{OgmiosRequest, OgmiosResponse, ogmios_request};
use anyhow::{Context, anyhow};
use inquire::InquireError;
use inquire::error::InquireResult;
use ogmios_client::jsonrpsee::{OgmiosClients, client_for_url};
use sp_core::offchain::Timestamp;
use std::{
	fs,
	io::{BufRead, BufReader, Read},
	path::PathBuf,
	process::Stdio,
	time::Duration,
};
use tempfile::TempDir;

pub trait IOContext {
	/// It should implement all the required traits for offchain operations
	type Offchain: GetScriptsData
		+ InitGovernance
		+ GetDParam
		+ UpsertDParam
		+ Deregister
		+ Register
		+ GetPermissionedCandidates
		+ UpsertPermissionedCandidates;

	fn run_command(&self, cmd: &str) -> anyhow::Result<String>;
	fn current_executable(&self) -> anyhow::Result<String>;
	fn print(&self, msg: &str);
	fn eprint(&self, msg: &str);
	fn enewline(&self);
	fn prompt(&self, prompt: &str, default: Option<&str>) -> String;
	fn prompt_yes_no(&self, prompt: &str, default: bool) -> bool;
	// TODO: 	fn prompt_multi_option<T: ToString>(&self, msg: &str, options: Vec<T>) -> T;
	fn prompt_multi_option(&self, msg: &str, options: Vec<String>) -> String;
	fn write_file(&self, path: &str, content: &str);

	fn new_tmp_dir(&self) -> PathBuf;
	fn read_file(&self, path: &str) -> Option<String>;
	fn file_exists(&self, path: &str) -> bool;
	fn list_directory(&self, path: &str) -> anyhow::Result<Option<Vec<String>>>;
	fn delete_file(&self, path: &str) -> anyhow::Result<()>;
	fn set_env_var(&self, key: &str, value: &str);
	fn current_timestamp(&self) -> Timestamp;
	fn ogmios_rpc(
		&self,
		config: &ServiceConfig,
		req: OgmiosRequest,
	) -> anyhow::Result<OgmiosResponse>;
	fn offchain_impl(&self, ogmios_config: &ServiceConfig) -> anyhow::Result<Self::Offchain>;
	fn config_file_path(&self, file: ConfigFile) -> String;

	fn chain_config_file_path(&self) -> String {
		self.config_file_path(ConfigFile::Chain)
	}
}

/// Default context implementation using standard IO.
pub struct DefaultCmdRunContext;

impl IOContext for DefaultCmdRunContext {
	// Currently WsClient implements Ogmios traits, that implement all required Offchain traits
	type Offchain = OgmiosClients;

	fn run_command(&self, cmd: &str) -> anyhow::Result<String> {
		eprintln!("running external command: {cmd}");

		let mut child = std::process::Command::new("sh")
			.arg("-c")
			.arg(cmd)
			.stderr(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn()
			.with_context(|| format!("Running executable failed: {cmd}"))?;

		// pass stderr, appending a prefix
		for line in BufReader::new(
			(child.stderr.as_mut()).context("Failed to read child process error output stream")?,
		)
		.lines()
		{
			let line = line.context("Failed to read error output line")?;
			self.eprint(&format!("command output: {line}"));
		}

		// capture stdout
		let mut output = vec![];
		child
			.stdout
			.as_mut()
			.context("Failed to read child process output stream")?
			.read_to_end(&mut output)
			.context("Failed to read child process output stream")?;

		let status = child.wait()?;
		if !status.success() {
			self.eprint(&format!("Running executable failed with status {}", status));
			if let Some(127) = status.code() {
				self.eprint("Make sure all executables are in path")
			}
			return Err(anyhow!("Failed to run command"));
		}
		Ok(String::from_utf8(output)?)
	}

	fn current_executable(&self) -> anyhow::Result<String> {
		let exe = std::env::current_exe()?;
		let node_executable = exe.to_str().ok_or(anyhow!("Cannot get current executable name"))?;
		Ok(node_executable.to_string())
	}

	fn print(&self, msg: &str) {
		println!("{msg}")
	}

	fn eprint(&self, msg: &str) {
		eprintln!("{msg}")
	}

	fn enewline(&self) {
		eprintln!()
	}

	fn prompt(&self, prompt: &str, default: Option<&str>) -> String {
		let mut prompt = inquire::Text::new(prompt);
		if let Some(default) = default {
			prompt = prompt.with_default(default)
		};

		handle_inquire_result(prompt.prompt())
	}

	fn prompt_yes_no(&self, prompt: &str, default: bool) -> bool {
		handle_inquire_result(inquire::Confirm::new(prompt).with_default(default).prompt())
	}

	fn prompt_multi_option(&self, msg: &str, options: Vec<String>) -> String {
		handle_inquire_result(inquire::Select::new(msg, options).prompt()).to_string()
	}

	fn write_file(&self, path: &str, content: &str) {
		fs::write(path, content).unwrap_or_else(|_| panic!("Failed to write file: {path}"))
	}

	fn new_tmp_dir(&self) -> PathBuf {
		TempDir::new().expect("Failed to create temporary directory").keep()
	}

	fn read_file(&self, path: &str) -> Option<String> {
		Some(
			String::from_utf8(fs::read(path).ok()?)
				.unwrap_or_else(|_| panic!("Failed to convert file from UTF-8: {path}")),
		)
	}

	fn file_exists(&self, path: &str) -> bool {
		fs::metadata(path).is_ok()
	}
	fn list_directory(&self, path: &str) -> anyhow::Result<Option<Vec<String>>> {
		if !self.file_exists(path) {
			return Ok(None);
		}

		let file_names = fs::read_dir(path)?
			.flat_map(|file| -> Option<_> { file.ok()?.file_name().into_string().ok() })
			.collect();

		Ok(Some(file_names))
	}
	fn delete_file(&self, path: &str) -> anyhow::Result<()> {
		fs::remove_file(path).context(format!("Failed to delete file: {path}"))
	}

	fn set_env_var(&self, key: &str, value: &str) {
		unsafe {
			std::env::set_var(key, value);
		}
	}

	fn current_timestamp(&self) -> Timestamp {
		let now = std::time::SystemTime::now();
		let duration = now
			.duration_since(std::time::SystemTime::UNIX_EPOCH)
			.expect("Current time is always after unix epoch");
		Timestamp::from_unix_millis(duration.as_millis() as u64)
	}

	fn ogmios_rpc(
		&self,
		config: &ServiceConfig,
		req: OgmiosRequest,
	) -> anyhow::Result<OgmiosResponse> {
		ogmios_request(config, req)
	}

	fn offchain_impl(&self, ogmios_config: &ServiceConfig) -> anyhow::Result<Self::Offchain> {
		let ogmios_address = ogmios_config.url();
		let tokio_runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
		tokio_runtime
			.block_on(client_for_url(
				&ogmios_address,
				Duration::from_secs(ogmios_config.timeout_seconds),
			))
			.map_err(|_| {
				anyhow!(format!("Couldn't open connection to Ogmios at {}", ogmios_address))
			})
	}

	fn config_file_path(&self, file: ConfigFile) -> String {
		match file {
			ConfigFile::Chain => {
				std::env::var("PC_CHAIN_CONFIG_PATH").unwrap_or("pc-chain-config.json".to_owned())
			},
			ConfigFile::Resources => std::env::var("PC_RESOURCES_CONFIG_PATH")
				.unwrap_or("pc-resources-config.json".to_owned()),
		}
	}
}

pub(crate) fn prompt_can_write<C: IOContext>(name: &str, path: &str, context: &C) -> bool {
	!context.file_exists(path)
		|| context.prompt_yes_no(&format!("{name} {path} exists - overwrite it?"), false)
}

fn handle_inquire_result<T>(result: InquireResult<T>) -> T {
	match result {
		Ok(result) => result,
		Err(InquireError::OperationInterrupted) => {
			eprintln!("Ctrl-C pressed. Exiting Wizard.");
			std::process::exit(0)
		},
		Err(InquireError::OperationCanceled) => std::process::exit(0),
		result => result.unwrap(),
	}
}
