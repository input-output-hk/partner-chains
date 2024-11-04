use crate::io::IOContext;
use crate::ogmios::{OgmiosRequest, OgmiosResponse};
use chain_params::SidechainParams;
use partner_chains_cardano_offchain::scripts_data::{GetScriptsData, ScriptsData};
use partner_chains_cardano_offchain::OffchainError;
use pretty_assertions::assert_eq;
use sp_core::offchain::Timestamp;
use std::collections::HashMap;
use std::panic::{catch_unwind, resume_unwind, UnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::TempPath;

mod config;

#[derive(Debug)]
#[allow(dead_code)]
pub enum MockIO {
	RunCommand { expected_cmd: String, output: anyhow::Result<String> },
	Print(String),
	EPrint(String),
	Prompt { prompt: String, default: Option<String>, input: String },
	PromptYN { prompt: String, default: bool, choice: bool },
	PromptMultiOption { prompt: String, options: Vec<String>, choice: String },
	FileRead { path: String },
	FileWrite { path: String, input: String },
	FileWriteJson { path: String, input: serde_json::Value },
	FileWriteJsonField { path: String, key: String, value: String },
	NewTmpFile { content: String },
	NewTmpDir,
	ListDirectory { path: String, result: Option<Vec<String>> },
	DeleteFile { path: String },
	SetEnvVar { key: String, value: String },
	SystemTimeNow(Timestamp),
	Group(Vec<MockIO>),
	WithFileLocation(&'static str, u32, Box<MockIO>),
	OgmiosRPC { addr: &'static str, req: OgmiosRequest, res: anyhow::Result<OgmiosResponse> },
}

impl MockIO {
	pub fn print_mock_location_on_panic<T>(self, f: impl Fn(MockIO) -> T + UnwindSafe) -> T {
		match self {
			MockIO::WithFileLocation(file, line, io) => {
				let result = catch_unwind(move || f(*io));
				match result {
					Ok(r) => r,
					Err(err) => {
						eprintln!("Mock IO defined at {file}:{line}");
						resume_unwind(err)
					},
				}
			},
			_ => f(self),
		}
	}
}

#[allow(dead_code)]
impl MockIO {
	#[track_caller]
	pub fn print(msg: &str) -> Self {
		Self::Print(msg.into()).with_location()
	}
	#[track_caller]
	pub fn eprint(msg: &str) -> Self {
		Self::EPrint(msg.into()).with_location()
	}
	#[track_caller]
	pub fn enewline() -> Self {
		Self::eprint("")
	}
	#[track_caller]
	pub fn prompt(prompt: &str, default: Option<&str>, input: &str) -> Self {
		Self::Prompt {
			prompt: prompt.into(),
			default: default.map(|s| s.into()),
			input: input.into(),
		}
		.with_location()
	}
	#[track_caller]
	pub fn file_read(path: &str) -> Self {
		Self::FileRead { path: path.into() }.with_location()
	}
	#[track_caller]
	pub fn file_write(path: &str, input: &str) -> Self {
		Self::FileWrite { path: path.into(), input: input.into() }.with_location()
	}

	#[track_caller]
	pub fn file_write_json_contains(path: &str, key: &str, value: &str) -> Self {
		Self::FileWriteJsonField { path: path.into(), key: key.into(), value: value.into() }
			.with_location()
	}

	#[track_caller]
	pub fn file_write_json(path: &str, input: serde_json::Value) -> Self {
		Self::FileWriteJson { path: path.into(), input }.with_location()
	}

	#[track_caller]
	pub fn new_tmp_file(content: &str) -> Self {
		Self::NewTmpFile { content: content.into() }.with_location()
	}

	#[track_caller]
	pub fn new_tmp_dir() -> Self {
		Self::NewTmpDir.with_location()
	}

	#[track_caller]
	pub fn delete_file(path: &str) -> Self {
		Self::DeleteFile { path: path.into() }.with_location()
	}
	#[track_caller]
	pub fn run_command(expected_cmd: &str, output: &str) -> Self {
		Self::RunCommand { expected_cmd: expected_cmd.into(), output: Ok(output.into()) }
			.with_location()
	}
	#[track_caller]
	pub fn run_command_with_result(
		expected_cmd: &str,
		output: anyhow::Result<std::string::String>,
	) -> Self {
		Self::RunCommand { expected_cmd: expected_cmd.into(), output }.with_location()
	}
	#[track_caller]
	pub fn run_command_json(expected_cmd: &str, output: &serde_json::Value) -> Self {
		Self::run_command(expected_cmd, &serde_json::to_string_pretty(output).unwrap())
	}
	#[track_caller]
	pub fn prompt_yes_no(prompt: &str, default: bool, choice: bool) -> Self {
		Self::PromptYN { prompt: prompt.into(), default, choice }.with_location()
	}
	#[track_caller]
	pub fn list_dir(path: &str, result: Option<Vec<String>>) -> Self {
		let path = path.into();
		Self::ListDirectory { path, result }.with_location()
	}
	#[track_caller]
	pub fn prompt_multi_option(prompt: &str, options: Vec<String>, choice: &str) -> Self {
		Self::PromptMultiOption { prompt: prompt.into(), options, choice: choice.into() }
			.with_location()
	}

	#[track_caller]
	pub fn set_env_var(key: &str, value: &str) -> Self {
		Self::SetEnvVar { key: key.into(), value: value.into() }.with_location()
	}

	#[track_caller]
	pub fn ogmios_request(
		addr: &'static str,
		req: OgmiosRequest,
		res: anyhow::Result<OgmiosResponse>,
	) -> Self {
		Self::OgmiosRPC { addr, req, res }
	}

	#[track_caller]
	fn with_location(self) -> Self {
		let loc = std::panic::Location::caller();
		MockIO::WithFileLocation(loc.file(), loc.line(), Box::new(self))
	}

	pub fn current_timestamp(time: Timestamp) -> Self {
		Self::SystemTimeNow(time)
	}
}

pub struct MockIOContext {
	pub expected_io: Arc<Mutex<Vec<MockIO>>>,
	pub files: Arc<Mutex<HashMap<String, String>>>,
	pub offchain_mocks: OffchainMocks,
}

impl MockIOContext {
	pub fn new() -> Self {
		Self {
			expected_io: Default::default(),
			files: Arc::new(Mutex::new(HashMap::default())),
			offchain_mocks: Default::default(),
		}
	}
	pub fn with_file(self, path: &str, content: &str) -> Self {
		self.files.lock().unwrap().insert(path.into(), content.into());
		self
	}
	pub fn with_json_file(self, path: &str, content: serde_json::Value) -> Self {
		let content = serde_json::to_string_pretty(&content).unwrap();
		self.with_file(path, &content)
	}
	pub fn with_expected_io(self, mut expected_commands: Vec<MockIO>) -> Self {
		expected_commands.reverse();
		let expected_commands = Arc::new(Mutex::new(expected_commands.into()));
		Self {
			expected_io: expected_commands,
			files: self.files.clone(),
			offchain_mocks: self.offchain_mocks.clone(),
		}
	}
	pub fn pop_next_action(&self, description: &str) -> MockIO {
		let next = self.expected_io.lock().unwrap().pop();
		match next {
			Some(MockIO::Group(mut group)) => {
				group.reverse();
				self.expected_io.lock().unwrap().append(&mut group);
				self.pop_next_action(description)
			},
			Some(other) => other,
			None => panic!("No more IO expected, but {description} called"),
		}
	}

	pub fn with_offchain_mocks(self, mocks: OffchainMocks) -> Self {
		Self {
			offchain_mocks: mocks,
			files: self.files.clone(),
			expected_io: self.expected_io.clone(),
		}
	}
}

// The only external dependnecy of Offchain is Ogmios. This key is Ogmios address.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct OffchainMockKey {
	ogmios_addr: String,
}

impl From<&str> for OffchainMockKey {
	fn from(value: &str) -> Self {
		Self { ogmios_addr: value.into() }
	}
}

#[derive(Default, Clone)]
pub(crate) struct OffchainMocks {
	mocks: HashMap<OffchainMockKey, OffchainMock>,
}

impl OffchainMocks {
	pub(crate) fn new_with_mock(addr: &str, mock: OffchainMock) -> Self {
		Self { mocks: vec![(addr.into(), mock)].into_iter().collect() }
	}
}

#[derive(Default, Clone)]
pub struct OffchainMock {
	pub scripts_data: HashMap<SidechainParams, Result<ScriptsData, OffchainError>>,
}

impl OffchainMock {
	pub(crate) fn new_with_scripts_data(
		pc_params: SidechainParams,
		scripts_data: Result<ScriptsData, OffchainError>,
	) -> Self {
		Self { scripts_data: vec![(pc_params, scripts_data)].into_iter().collect() }
	}
}

impl GetScriptsData for OffchainMock {
	async fn get_scripts_data(
		&self,
		pc_params: SidechainParams,
	) -> Result<ScriptsData, OffchainError> {
		self.scripts_data.get(&pc_params).cloned().unwrap_or_else(|| {
			Err(OffchainError::InternalError("No mock for shelley_genesis_configuration".into()))
		})
	}
}

impl Drop for MockIOContext {
	fn drop(&mut self) {
		if std::thread::panicking() {
			// the test has already failed, do not panic again
			return;
		}
		if let Some(next_expected) = self.expected_io.lock().unwrap().first() {
			panic!("IO operations left unperformed. Next expected: {:?}", next_expected);
		}
	}
}

impl IOContext for MockIOContext {
	type Offchain = OffchainMock;

	fn run_command(&self, cmd: &str) -> anyhow::Result<String> {
		let next = self.pop_next_action(&format!("run_command({cmd})"));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::RunCommand { expected_cmd, output } => {
				assert_eq!(
					cmd, expected_cmd,
					"Incorrect command executed: {cmd}  expected: {expected_cmd}"
				);
				output
			},
			other => panic!("Unexpected command executed: {cmd}, expected: {other:?}"),
		})
	}

	fn eprint(&self, msg: &str) {
		let next = self.pop_next_action(&format!("eprint({msg})"));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::EPrint(expected_msg) => {
				assert_eq!(msg, expected_msg, "Incorrect message printed")
			},
			other => panic!("Unexpected stderr message printed: {msg}, expected: {other:?}"),
		})
	}

	fn enewline(&self) {
		self.eprint("")
	}

	fn print(&self, msg: &str) {
		let next = self.pop_next_action(&format!("print({msg})"));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::Print(expected_msg) => {
				assert_eq!(msg, expected_msg, "Incorrect message printed")
			},
			other => panic!("Unexpected message printed: {msg}, expected: {other:?}"),
		})
	}

	fn prompt(&self, prompt: &str, default: Option<&str>) -> String {
		let next =
			self.pop_next_action(&format!("prompt(prompt = {prompt}, default = {default:?})"));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::Prompt { prompt: expected_prompt, default: expected_default, input } => {
				assert_eq!(prompt, expected_prompt, "Invalid prompt displayed");
				assert_eq!(
					default.map(|s| s.into()),
					expected_default,
					"Invalid default value for prompt"
				);
				input
			},
			other => panic!("Unexpected prompt displayed: {prompt}, expected: {other:?}"),
		})
	}

	fn write_file(&self, path: &str, input: &str) {
		let next = self.pop_next_action(&format!("write_file(path = {path}, input = {input})"));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::FileWriteJsonField {
				path: expected_path,
				key: expected_key,
				value: expected_value,
			} => {
				assert_eq!(
					path, expected_path,
					"Unexpected file write: {path}, expected: {expected_path}"
				);
				let parsed_input: serde_json::Value =
					serde_json::from_str(input).expect("Invalid json write attempt to {path}");
				let value_opt = parsed_input.pointer(&expected_key);
				let value = value_opt.unwrap_or_else(|| {
					panic!("Unexpected write input. Expected key {expected_key} not found in json {parsed_input}")
				});

				assert_eq!(
					expected_value,
					value.to_string().replace('"', ""),
					"Unexpected write input: {value}, expected: {expected_value}"
				);
				self.files.lock().unwrap().insert(path.into(), input.into());
			},
			MockIO::FileWriteJson { path: expected_path, input: expected_input } => {
				assert_eq!(
					path, expected_path,
					"Unexpected file write: {path}, expected: {expected_path}"
				);
				let parsed_input: serde_json::Value =
					serde_json::from_str(input).expect("Invalid json write attempt to {path}");
				assert_eq!(
					parsed_input, expected_input,
					"Unexpected write input: {parsed_input}, expected: {expected_input}"
				);
				self.files.lock().unwrap().insert(path.into(), input.into());
			},
			MockIO::FileWrite { path: expected_path, input: expected_input } => {
				assert_eq!(
					path, expected_path,
					"Unexpected file write: {path}, expected: {expected_path}"
				);
				assert_eq!(
					input, &expected_input,
					"Unexpected file write input: {input}, expected: {expected_input}"
				);
				self.files.lock().unwrap().insert(path.into(), input.into());
			},
			other => panic!("Unexpected file write action, expected: {other:?}"),
		})
	}

	fn read_file(&self, path: &str) -> Option<String> {
		let next = self.pop_next_action(&format!("read_file({path})"));
		let content = self.files.lock().unwrap().get::<String>(&path.to_string()).cloned();
		next.print_mock_location_on_panic(|next| match next {
			MockIO::FileRead { path: expected_path } => {
				assert_eq!(
					path, expected_path,
					"File read for incorrect file {path}, expected: {expected_path}"
				);
				content.clone()
			},
			other => panic!("Unexpected file read for {path}, expected: {other:?}"),
		})
	}

	fn file_exists(&self, path: &str) -> bool {
		self.files.lock().unwrap().get::<String>(&path.to_string()).is_some()
	}

	fn prompt_yes_no(&self, prompt: &str, default: bool) -> bool {
		let next =
			self.pop_next_action(&format!("prompt_yes_no(prompt = {prompt}, default = {default})"));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::PromptYN { prompt: expected_prompt, default: expected_default, choice } => {
				assert_eq!(prompt, expected_prompt);
				assert_eq!(default, expected_default);
				choice
			},
			other => panic!("Unexpected Y/N prompt, expected: {other:?}"),
		})
	}

	fn list_directory(&self, path: &str) -> anyhow::Result<Option<Vec<String>>> {
		let next = self.pop_next_action(&format!("list_directory({path})"));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::ListDirectory { path: expected_path, result } => {
				assert_eq!(
					path, expected_path,
					"Incorrect directory listed: {path}, expected: {expected_path}"
				);
				Ok(result)
			},
			other => panic!("Unexpected directory listing for {path}. Expected: {other:?}"),
		})
	}

	fn delete_file(&self, path: &str) -> anyhow::Result<()> {
		let next = self.pop_next_action(&format!("delete_file({path})"));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::DeleteFile { path: expected_path } => {
				assert_eq!(
					path, expected_path,
					"Incorrect file delete: {path}, expected: {expected_path}"
				);
				Ok(())
			},
			other => panic!("Unexpected file deletion of {path}, expected: {other:?}"),
		})
	}

	fn prompt_multi_option(&self, prompt: &str, options: Vec<String>) -> String {
		let next = self.pop_next_action(&format!(
			"prompt_multi_option(prompt = {prompt}, options = {options:?})",
		));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::PromptMultiOption {
				prompt: expected_prompt,
				options: expected_options,
				choice,
			} => {
				assert_eq!(prompt, expected_prompt);
				assert_eq!(options, expected_options);
				choice
			},
			other => panic!("Unexpected multi-option prompt, expected: {other:?}"),
		})
	}

	fn set_env_var(&self, key: &str, value: &str) {
		let next = self.pop_next_action(&format!("set_env_var(key = {key}, value = {value})"));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::SetEnvVar { key: expected_key, value: expected_value } => {
				assert_eq!(key, expected_key, "Invalid env var key");
				assert_eq!(value, expected_value, "Invalid env var value");
			},
			other => panic!("Unexpected env var set: {key}={value}, expected: {other:?}"),
		})
	}

	fn current_timestamp(&self) -> Timestamp {
		let next = self.pop_next_action(&format!("current_timestamp()"));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::SystemTimeNow(time) => time,
			other => panic!("Unexpected system time request, expected: {other:?}"),
		})
	}
	fn new_tmp_file(&self, content: &str) -> TempPath {
		let next = self.pop_next_action(&format!("new_tmp_file(content = {content})"));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::NewTmpFile { content: expected_content } => {
				assert_eq!(
					content, expected_content,
					"Unexpected file write: {content}, expected content: {expected_content}"
				);
				let path = format!("/tmp/dummy{}", self.files.lock().unwrap().len());
				self.files.lock().unwrap().insert(path.clone(), content.into());
				TempPath::from_path(Path::new(&path))
			},
			other => panic!("Unexpected new tmp file action, expected: {other:?}"),
		})
	}

	fn new_tmp_dir(&self) -> PathBuf {
		let next = self.pop_next_action("new_tmp_dir()");
		next.print_mock_location_on_panic(|next| match next {
			MockIO::NewTmpDir => PathBuf::from("/tmp/MockIOContext_tmp_dir"),
			other => {
				panic!("Unexpected new temporary directory request, expected: {other:?}")
			},
		})
	}

	fn ogmios_rpc(
		&self,
		addr: &str,
		req: crate::ogmios::OgmiosRequest,
	) -> anyhow::Result<crate::ogmios::OgmiosResponse> {
		let next = self.pop_next_action(&format!("ogmios_rpc(addr = {addr}, req = {req:?})"));
		next.print_mock_location_on_panic(|next| match next {
			MockIO::OgmiosRPC { addr: expected_addr, req: expected_req, res } => {
				assert_eq!(addr, expected_addr, "Unexpected Ogmios RPC address");
				assert_eq!(req, expected_req, "Unexpected Ogmios RPC request");
				res
			},
			other => panic!("Unexpected Ogmios RPC request, expected: {other:?}"),
		})
	}

	fn offchain_impl(
		&self,
		ogmios_config: &crate::config::ServiceConfig,
	) -> anyhow::Result<Self::Offchain> {
		let addr: &str = &ogmios_config.to_string();
		let mock = self.offchain_mocks.mocks.get(&addr.into()).ok_or_else(|| {
			anyhow::anyhow!("No mock for Offchain implementation for {:?}", ogmios_config)
		})?;
		Ok(mock.clone())
	}
}

#[test]
fn verify_cli() {
	use clap::CommandFactory;
	crate::Command::command().debug_assert()
}
