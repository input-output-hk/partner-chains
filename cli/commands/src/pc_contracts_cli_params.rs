use clap::Parser;

#[derive(Clone, Debug, Default, Parser)]
pub struct PcContractsCliParams {
	#[arg(long)]
	pub ogmios_host: Option<String>,
	#[arg(long)]
	pub ogmios_port: Option<u64>,
	#[arg(long, default_value = "false")]
	pub ogmios_secure: bool,

	#[arg(long)]
	pub kupo_host: Option<String>,
	#[arg(long)]
	pub kupo_port: Option<u64>,
	#[arg(long, default_value = "false")]
	pub kupo_secure: bool,
}

impl PcContractsCliParams {
	pub fn command_line_params(&self) -> Vec<Vec<String>> {
		let mut args = vec![];
		if let Some(ogmios_host) = &self.ogmios_host {
			args.push(vec![format!("--ogmios-host {ogmios_host}")])
		}
		if let Some(ogmios_port) = &self.ogmios_port {
			args.push(vec![format!("--ogmios-port {ogmios_port}")])
		}
		if self.ogmios_secure {
			args.push(vec!["--ogmios-secure".into()])
		}
		if let Some(kupo_host) = &self.kupo_host {
			args.push(vec![format!("--kupo-host {kupo_host}")])
		}
		if let Some(kupo_port) = &self.kupo_port {
			args.push(vec![format!("--kupo-port {kupo_port}")])
		}
		if self.kupo_secure {
			args.push(vec!["--kupo-secure".into()])
		}
		args
	}
}
