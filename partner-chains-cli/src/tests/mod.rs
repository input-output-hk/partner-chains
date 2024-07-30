use crate::io::IOContext;
use pretty_assertions::assert_eq;
use sp_core::offchain::Timestamp;
use std::path::{Path, PathBuf};
use std::{cell::RefCell, collections::HashMap};
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
}

#[allow(dead_code)]
impl MockIO {
	pub fn print(msg: &str) -> Self {
		Self::Print(msg.into())
	}
	pub fn eprint(msg: &str) -> Self {
		Self::EPrint(msg.into())
	}
	pub fn enewline() -> Self {
		Self::eprint("")
	}
	pub fn prompt(prompt: &str, default: Option<&str>, input: &str) -> Self {
		Self::Prompt {
			prompt: prompt.into(),
			default: default.map(|s| s.into()),
			input: input.into(),
		}
	}
	pub fn file_read(path: &str) -> Self {
		Self::FileRead { path: path.into() }
	}
	pub fn file_write(path: &str, input: &str) -> Self {
		Self::FileWrite { path: path.into(), input: input.into() }
	}

	pub fn file_write_json_contains(path: &str, key: &str, value: &str) -> Self {
		Self::FileWriteJsonField { path: path.into(), key: key.into(), value: value.into() }
	}

	pub fn file_write_json(path: &str, input: serde_json::Value) -> Self {
		Self::FileWriteJson { path: path.into(), input }
	}

	pub fn new_tmp_file(content: &str) -> Self {
		Self::NewTmpFile { content: content.into() }
	}

	pub fn new_tmp_dir() -> Self {
		Self::NewTmpDir
	}

	pub fn delete_file(path: &str) -> Self {
		Self::DeleteFile { path: path.into() }
	}
	pub fn run_command(expected_cmd: &str, output: &str) -> Self {
		Self::RunCommand { expected_cmd: expected_cmd.into(), output: Ok(output.into()) }
	}
	pub fn run_command_json(expected_cmd: &str, output: &serde_json::Value) -> Self {
		Self::run_command(expected_cmd, &serde_json::to_string_pretty(output).unwrap())
	}
	pub fn prompt_yes_no(prompt: &str, default: bool, choice: bool) -> Self {
		Self::PromptYN { prompt: prompt.into(), default, choice }
	}
	pub fn list_dir(path: &str, result: Option<Vec<String>>) -> Self {
		let path = path.into();
		Self::ListDirectory { path, result }
	}
	pub fn prompt_multi_option(prompt: &str, options: Vec<String>, choice: &str) -> Self {
		Self::PromptMultiOption { prompt: prompt.into(), options, choice: choice.into() }
	}
	pub fn set_env_var(key: &str, value: &str) -> Self {
		Self::SetEnvVar { key: key.into(), value: value.into() }
	}

	pub fn current_timestamp(time: Timestamp) -> Self {
		Self::SystemTimeNow(time)
	}
}

pub struct MockIOContext {
	expected_io: RefCell<Vec<MockIO>>,
	files: RefCell<HashMap<String, String>>,
}

impl MockIOContext {
	pub fn new() -> Self {
		Self { expected_io: vec![].into(), files: HashMap::default().into() }
	}
	pub fn with_file(self, path: &str, content: &str) -> Self {
		self.files.borrow_mut().insert(path.into(), content.into());
		self
	}
	pub fn with_json_file(self, path: &str, content: serde_json::Value) -> Self {
		let content = serde_json::to_string_pretty(&content).unwrap();
		self.with_file(path, &content)
	}
	pub fn with_expected_io(self, mut expected_commands: Vec<MockIO>) -> Self {
		expected_commands.reverse();
		let expected_commands = expected_commands.into();
		Self { expected_io: expected_commands, ..self }
	}
	pub fn pop_next_action(&self) -> Option<MockIO> {
		let next = self.expected_io.borrow_mut().pop();
		match next {
			Some(MockIO::Group(mut group)) => {
				group.reverse();
				self.expected_io.borrow_mut().append(&mut group);
				self.pop_next_action()
			},
			Some(other) => Some(other),
			None => None,
		}
	}
	pub fn no_more_io_expected(&self) {
		assert!(
			self.expected_io.borrow().is_empty(),
			"Expected IO operations left unperformed: {:?}",
			self.expected_io
		)
	}
}

impl IOContext for MockIOContext {
	fn run_command(&self, cmd: &str) -> anyhow::Result<String> {
		match self.pop_next_action() {
			Some(MockIO::RunCommand { expected_cmd, output }) => {
				assert_eq!(
					cmd, expected_cmd,
					"Incorrect command executed: {cmd}  expected: {expected_cmd}"
				);
				output
			},
			Some(other) => panic!("Unexpected command executed: {cmd}, expected: {other:?}"),
			None => panic!("Unexpected command executed: {cmd}, expected no more content actions"),
		}
	}

	fn eprint(&self, msg: &str) {
		match self.pop_next_action() {
			Some(MockIO::EPrint(expected_msg)) => {
				assert_eq!(msg, expected_msg, "Incorrect message printed")
			},
			Some(other) => panic!("Unexpected stderr message printed: {msg}, expected: {other:?}"),
			None => {
				panic!("Unexpected stderr message printed: {msg}, expected no more content actions")
			},
		}
	}

	fn enewline(&self) {
		self.eprint("")
	}

	fn print(&self, msg: &str) {
		match self.pop_next_action() {
			Some(MockIO::Print(expected_msg)) => {
				assert_eq!(msg, expected_msg, "Incorrect message printed")
			},
			Some(other) => panic!("Unexpected message printed: {msg}, expected: {other:?}"),
			None => {
				panic!("Unexpected stdout message printed: {msg}, expected no more content actions")
			},
		}
	}

	fn prompt(&self, prompt: &str, default: Option<&str>) -> String {
		match self.pop_next_action() {
			Some(MockIO::Prompt { prompt: expected_prompt, default: expected_default, input }) => {
				assert_eq!(prompt, expected_prompt, "Invalid prompt displayed");
				assert_eq!(
					default.map(|s| s.into()),
					expected_default,
					"Invalid default value for prompt"
				);
				input
			},
			Some(other) => panic!("Unexpected prompt displayed: {prompt}, expected: {other:?}"),
			None => panic!("Unexpected prompt displayed: {prompt}, no more actions expected"),
		}
	}

	fn write_file(&self, path: &str, input: &str) {
		match self.pop_next_action() {
			Some(MockIO::FileWriteJsonField {
				path: expected_path,
				key: expected_key,
				value: expected_value,
			}) => {
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
				self.files.borrow_mut().insert(path.into(), input.into());
			},
			Some(MockIO::FileWriteJson { path: expected_path, input: expected_input }) => {
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
				self.files.borrow_mut().insert(path.into(), input.into());
			},
			Some(MockIO::FileWrite { path: expected_path, input: expected_input }) => {
				assert_eq!(
					path, expected_path,
					"Unexpected file write: {path}, expected: {expected_path}"
				);
				assert_eq!(
					input, &expected_input,
					"Unexpected file write input: {input}, expected: {expected_input}"
				);
				self.files.borrow_mut().insert(path.into(), input.into());
			},
			Some(other) => panic!("Unexpected file write action, expected: {other:?}"),
			None => panic!("Unexpected file write action, no more actions expected"),
		}
	}

	fn read_file(&self, path: &str) -> Option<String> {
		match self.pop_next_action() {
			Some(MockIO::FileRead { path: expected_path }) => {
				assert_eq!(
					path, &expected_path,
					"File read for incorrect file {path}, expected: {expected_path}"
				);
				self.files.borrow_mut().get::<String>(&path.to_string()).cloned()
			},
			Some(other) => panic!("Unexpected file read for {path}, expected: {other:?}"),
			None => panic!("Unexpected file read for {path}, no more actions expected"),
		}
	}

	fn file_exists(&self, path: &str) -> bool {
		self.files.borrow_mut().get::<String>(&path.to_string()).is_some()
	}

	fn prompt_yes_no(&self, prompt: &str, default: bool) -> bool {
		match self.pop_next_action() {
			Some(MockIO::PromptYN {
				prompt: expected_prompt,
				default: expected_default,
				choice,
			}) => {
				assert_eq!(prompt, expected_prompt);
				assert_eq!(default, expected_default);
				choice
			},
			Some(other) => panic!("Unexpected Y/N prompt, expected: {other:?}"),
			None => panic!("Unexpected Y/N prompt: {prompt}, no more actions expected"),
		}
	}

	fn list_directory(&self, path: &str) -> anyhow::Result<Option<Vec<String>>> {
		match self.pop_next_action() {
			Some(MockIO::ListDirectory { path: expected_path, result }) => {
				assert_eq!(
					path, expected_path,
					"Incorrect directory listed: {path}, expected: {expected_path}"
				);
				Ok(result)
			},
			Some(other) => panic!("Unexpected directory listing for {path}. Expected: {other:?}"),
			None => panic!("Unexpected directory listing for {path}, no more actions expected"),
		}
	}

	fn delete_file(&self, path: &str) -> anyhow::Result<()> {
		match self.pop_next_action() {
			Some(MockIO::DeleteFile { path: expected_path }) => {
				assert_eq!(
					path, expected_path,
					"Incorrect file delete: {path}, expected: {expected_path}"
				);
				Ok(())
			},
			Some(other) => panic!("Unexpected file deletion of {path}, expected: {other:?}"),
			None => panic!("Unexpected file deletion of {path}, no more actions expected"),
		}
	}

	fn prompt_multi_option(&self, prompt: &str, options: Vec<String>) -> String {
		match self.pop_next_action() {
			Some(MockIO::PromptMultiOption {
				prompt: expected_prompt,
				options: expected_options,
				choice,
			}) => {
				assert_eq!(prompt, expected_prompt);
				assert_eq!(options, expected_options);
				choice
			},
			Some(other) => panic!("Unexpected multi-option prompt, expected: {other:?}"),
			None => panic!("Unexpected multi-option prompt: {prompt}, no more actions expected"),
		}
	}

	fn set_env_var(&self, key: &str, value: &str) {
		match self.pop_next_action() {
			Some(MockIO::SetEnvVar { key: expected_key, value: expected_value }) => {
				assert_eq!(key, expected_key, "Invalid env var key");
				assert_eq!(value, expected_value, "Invalid env var value");
			},
			Some(other) => panic!("Unexpected env var set: {key}={value}, expected: {other:?}"),
			None => panic!("Unexpected env var set: {key}={value}, no more actions expected"),
		}
	}

	fn current_timestamp(&self) -> Timestamp {
		match self.pop_next_action() {
			Some(MockIO::SystemTimeNow(time)) => time,
			Some(other) => panic!("Unexpected system time request, expected: {other:?}"),
			None => panic!("Unexpected system time request, no more actions expected"),
		}
	}
	fn new_tmp_file(&self, content: &str) -> TempPath {
		match self.pop_next_action() {
			Some(MockIO::NewTmpFile { content: expected_content }) => {
				assert_eq!(
					content, expected_content,
					"Unexpected file write: {content}, expected content: {expected_content}"
				);
				let path = format!("/tmp/dummy{}", self.files.borrow().len());
				self.files.borrow_mut().insert(path.clone(), content.into());
				TempPath::from_path(Path::new(&path))
			},
			Some(other) => panic!("Unexpected new tmp file action, expected: {other:?}"),
			None => panic!("Unexpected new tmp file action, no more actions expected"),
		}
	}

	fn new_tmp_dir(&self) -> PathBuf {
		match self.pop_next_action() {
			Some(MockIO::NewTmpDir) => PathBuf::from("/tmp/MockIOContext_tmp_dir"),
			Some(other) => {
				panic!("Unexpected new temporary directory request, expected: {other:?}")
			},
			None => panic!("Unexpected new temporary directory request, no more actions expected"),
		}
	}
}

#[test]
fn verify_cli() {
	use clap::CommandFactory;
	crate::Command::command().debug_assert()
}
