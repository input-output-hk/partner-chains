
use std::io::Read;

pub struct Runner {
	pub shell: String,
}
impl Runner {
	pub fn new(shell: String) -> Self {
		Runner { shell }
	}

	pub fn run(&self, raw_command: &str, _timeout: u64) -> Result<String, String> {
		use std::process::Stdio;

		let command = format!("{} \"{}\"", self.shell, raw_command.replace("\n", " "));
		log::info!("RUNNING: {command}");
		let mut child = std::process::Command::new("sh")
			.arg("-c")
			.arg(command)
			.stderr(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn()
			.map_err(|e| e.to_string())?;

		let mut stderr = String::new();
		std::io::BufReader::new(child.stderr.as_mut().ok_or("can't read stderr")?)
			.read_to_string(&mut stderr)
			.map_err(|e| e.to_string())?;

		let mut stdout = String::new();
		std::io::BufReader::new(child.stdout.as_mut().ok_or("can't read stdout")?)
			.read_to_string(&mut stdout)
			.map_err(|e| e.to_string())?;

		let status = child.wait().map_err(|e| e.to_string())?;
		if status.success() {
			Ok(stdout.trim().to_string())
		} else {
			Err(stderr.trim().to_string())
		}
	}
}
