use crate::config::ServiceConfig;
use crate::ogmios::{ogmios_request, OgmiosRequest, OgmiosResponse};
use anyhow::{anyhow, Context};
use jsonrpsee::http_client::HttpClient;
use partner_chains_cardano_offchain::scripts_data::GetScriptsData;
use sp_core::offchain::Timestamp;
use std::path::PathBuf;
use std::{
	fs,
	io::{BufRead, BufReader, Read},
	process::Stdio,
};
use tempfile::{TempDir, TempPath};

pub trait IOContext {
	/// It should implement all the required traits for offchain operations
	type Offchain: GetScriptsData;

	fn run_command(&self, cmd: &str) -> anyhow::Result<String>;
	fn print(&self, msg: &str);
	fn eprint(&self, msg: &str);
	fn enewline(&self);
	fn prompt(&self, prompt: &str, default: Option<&str>) -> String;
	fn prompt_yes_no(&self, prompt: &str, default: bool) -> bool;
	fn prompt_multi_option(&self, msg: &str, options: Vec<String>) -> String;
	fn write_file(&self, path: &str, content: &str);
	fn new_tmp_file(&self, content: &str) -> TempPath;

	fn new_tmp_dir(&self) -> PathBuf;
	fn read_file(&self, path: &str) -> Option<String>;
	fn file_exists(&self, path: &str) -> bool;
	fn list_directory(&self, path: &str) -> anyhow::Result<Option<Vec<String>>>;
	fn delete_file(&self, path: &str) -> anyhow::Result<()>;
	fn set_env_var(&self, key: &str, value: &str);
	fn current_timestamp(&self) -> Timestamp;
	fn ogmios_rpc(&self, addr: &str, req: OgmiosRequest) -> anyhow::Result<OgmiosResponse>;
	fn offchain_impl(&self, ogmios_config: &ServiceConfig) -> anyhow::Result<Self::Offchain>;
}

pub struct DefaultCmdRunContext;

impl IOContext for DefaultCmdRunContext {
	// Currently HttpClient implements Ogmios traits, that implement all required Offchain traits
	type Offchain = HttpClient;

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

		prompt.prompt().unwrap()
	}

	fn prompt_yes_no(&self, prompt: &str, default: bool) -> bool {
		inquire::Confirm::new(prompt).with_default(default).prompt().unwrap()
	}

	fn prompt_multi_option(&self, msg: &str, options: Vec<String>) -> String {
		inquire::Select::new(msg, options).prompt().unwrap().to_string()
	}

	fn write_file(&self, path: &str, content: &str) {
		fs::write(path, content).unwrap_or_else(|_| panic!("Failed to write file: {path}"))
	}

	fn new_tmp_file(&self, content: &str) -> TempPath {
		let file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
		self.write_file(
			file.path().to_str().expect("temporary file paths are expected to be unicode"),
			content,
		);
		file.into_temp_path()
	}

	fn new_tmp_dir(&self) -> PathBuf {
		TempDir::new().expect("Failed to create temporary directory").into_path()
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
		std::env::set_var(key, value);
	}

	fn current_timestamp(&self) -> Timestamp {
		let now = std::time::SystemTime::now();
		let duration = now
			.duration_since(std::time::SystemTime::UNIX_EPOCH)
			.expect("Current time is always after unix epoch");
		Timestamp::from_unix_millis(duration.as_millis() as u64)
	}

	fn ogmios_rpc(&self, addr: &str, req: OgmiosRequest) -> anyhow::Result<OgmiosResponse> {
		ogmios_request(addr, req)
	}

	fn offchain_impl(&self, ogmios_config: &ServiceConfig) -> anyhow::Result<Self::Offchain> {
		let ogmios_address = ogmios_config.to_string();
		HttpClient::builder().build(&ogmios_address).map_err(|_| {
			anyhow!(format!("Couldn't open connection to Ogmios at {}", ogmios_address))
		})
	}
}

pub fn prompt_can_write<C: IOContext>(name: &str, path: &str, context: &C) -> bool {
	!context.file_exists(path)
		|| context.prompt_yes_no(&format!("{name} {path} exists - overwrite it?"), false)
}
