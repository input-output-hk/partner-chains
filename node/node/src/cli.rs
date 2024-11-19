use clap::command;
use partner_chains_node_commands::PartnerChainsSubcommand;
use sc_cli::RunCmd;

#[derive(Debug, clap::Parser)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[clap(flatten)]
	pub run: RunCmd,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Key management cli utilities
	#[command(subcommand)]
	Key(sc_cli::KeySubcommand),

	#[clap(flatten)]
	PartnerChains(PartnerChainsSubcommand),

	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	/// Sub-commands concerned with benchmarking.
	#[command(subcommand)]
	Benchmark(frame_benchmarking_cli::BenchmarkCmd),

	/// Db meta columns information.
	ChainInfo(sc_cli::ChainInfoCmd),
}

#[cfg(test)]
mod registration_signatures_tests {
	use assert_cmd::Command;

	#[test]
	fn registration_signatures() {
		let mut cmd = Command::cargo_bin("partner-chains-node").unwrap();
		let cmd_result = cmd.args(REGISTRATION_SIGS_CMD.split(' ')).assert().success();
		let output = std::str::from_utf8(&cmd_result.get_output().stdout).unwrap();
		let output_json: serde_json::Value = serde_json::from_str(output).unwrap();
		assert_eq!(
			output_json,
			serde_json::json!({
				"spo_public_key": "0xfb335cabe7d3dd77d0177cd332e9a44998d9d5085b811650853b7bb0752a8bef",
				"spo_signature": "0x359bca8b2196f5618c14635419a523d22625253069e3963342af980542dcd49b737a9725f0a3970974d0566b44c6e069bcd89265d1f0fdc1c629b6eaa73b130d",
				"sidechain_public_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1",
				"sidechain_signature": "0xce7696304946a9eaec3c2c8be1aa49023f0fe01b08c7097c8493d733424f22fe45153632876785143fe4db8f362b2b1dfcede2c755d11c6a1b78a2f4b7f1b87d"
			})
		)
	}

	const REGISTRATION_SIGS_CMD: &str = "registration-signatures \
      --genesis-utxo f17e6d3aa72095e04489d13d776bf05a66b5a8c49d89397c28b18a1784b9950e#0 \
      --registration-utxo cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13#0 \
      --mainchain-signing-key 0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf \
      --sidechain-signing-key cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854";
}
