
use std::time::Duration;

use crate::blockchain_api::*;
use crate::run_command::*;
use serde_json::Value as JsonValue;

pub fn write_file(runner: &Runner, content: &str) -> Result<String, String> {
	// TODO clean up temp files
	let filepath = format!("/tmp/{}", uuid4());
	runner.run(&format!("echo '{content}' > {filepath}"), 120)?;
	Ok(filepath)
}

pub fn wait_until<F>(
	description: &str,
	condition: F,
	timeout: u64,
	poll_interval: u64,
) -> Result<String, String>
where
	F: Fn() -> Option<String>,
{
	let start = std::time::SystemTime::now();
	log::info!("WAIT UNTIL: {description}. TIMEOUT: {timeout}, POLL_INTERVAL: {poll_interval}");
	while std::time::SystemTime::now().duration_since(start).map_err(|e| e.to_string())?
		< Duration::from_secs(timeout)
	{
		if let Some(result) = condition() {
			return Ok(result);
		} else {
			std::thread::sleep(Duration::from_secs(poll_interval));
		}
	}
	Err(format!("WAIT UNTIL function TIMED OUT after {timeout}s on {description}."))
}

pub fn get_scripts(api: &SubstrateApi) -> JsonValue {
	api.partner_chains_node().get_scripts().unwrap()
}

pub fn addresses(api: &SubstrateApi) -> JsonValue {
	api.partner_chains_node().get_scripts().unwrap()["addresses"].clone()
}
