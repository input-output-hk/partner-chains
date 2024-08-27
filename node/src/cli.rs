use clap::command;
use sc_cli::RunCmd;
use chain_params::SidechainParams;
use partner_chains_node_commands::PartnerChainsSubcommand;

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
	PartnerChains(PartnerChainsSubcommand<SidechainParams>),

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
				"spo_signature": "0x1a55db596380bc63f5ee964565359b5ea8e0096c798c3281692df097abbd9aa4b657f887915ad2a52fc85c674ef4044baeaf7149546af93a2744c379b9798f07",
				"sidechain_public_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1",
				"sidechain_signature": "0x676094d8ae924afd157ebcd5b52852648502906d0739929a976dd371ca0e37c40839946255b35bb1ceb377e502a163bcc108aed1b2ee5e03165e7edd1b37c658"
			})
		)
	}

	const REGISTRATION_SIGS_CMD: &str = "registration-signatures \
      --genesis-committee-utxo f17e6d3aa72095e04489d13d776bf05a66b5a8c49d89397c28b18a1784b9950e#0 \
      --chain-id 0 \
      --threshold-numerator 2 \
      --threshold-denominator 3 \
      --governance-authority 00112233445566778899001122334455667788990011223344556677 \
      --registration-utxo cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13#0 \
      --mainchain-signing-key 0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf \
      --sidechain-signing-key cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854";
}
