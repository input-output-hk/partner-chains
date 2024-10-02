use crate::io::IOContext;
use crate::CmdRun;
use clap::Parser;
use cli_commands::key_params::{
	MainchainSigningKeyParam, PlainPublicKeyParam, SidechainPublicKeyParam,
};
use cli_commands::registration_signatures::RegisterValidatorMessage;
use cli_commands::signing::mainchain_public_key_and_signature;
use sidechain_domain::UtxoId;
use std::str::FromStr;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Register2Cmd {
	#[clap(flatten)]
	pub sidechain_params: chain_params::SidechainParams,
	#[arg(long)]
	pub registration_utxo: UtxoId,
	#[arg(long)]
	pub sidechain_pub_key: SidechainPublicKeyParam,
	#[arg(long)]
	pub aura_pub_key: PlainPublicKeyParam,
	#[arg(long)]
	pub grandpa_pub_key: PlainPublicKeyParam,
	#[arg(long)]
	pub sidechain_signature: String,
}

impl CmdRun for Register2Cmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		context.print("⚙️ Register as a committee candidate (step 2/3)");
		context.print(
			"  This command will use SPO cold signing key for signing the registration message.",
		);

		let mainchain_signing_key_path =
			context.prompt("Path to mainchain signing key file", Some("cold.skey"));
		let mainchain_signing_key = get_mainchain_cold_skey(context, &mainchain_signing_key_path)
			.map_err(|e| {
			context.eprint("Unable to read mainchain signing key file");
			e
		})?;

		let registration_message = RegisterValidatorMessage::<chain_params::SidechainParams> {
			sidechain_params: self.sidechain_params.clone(),
			sidechain_pub_key: self.sidechain_pub_key.0.clone(),
			input_utxo: self.registration_utxo,
		};

		let (verification_key, mc_signature) =
			mainchain_public_key_and_signature(mainchain_signing_key.0, registration_message);

		let spo_public_key = hex::encode(verification_key);
		let spo_signature = hex::encode(mc_signature.to_vec());
		let governance_authority = self.sidechain_params.governance_authority.to_hex_string();

		context.print("To finish the registration process, run the following command on the machine with the partner chain dependencies running:\n");
		context.print(&format!(
			"./partner-chains-cli register3 \\\n--governance-authority {} \\\n--genesis-committee-utxo {} \\\n--registration-utxo {} \\\n--aura-pub-key {} \\\n--grandpa-pub-key {} \\\n--sidechain-pub-key {} \\\n--sidechain-signature {} \\\n--spo-public-key {} \\\n--spo-signature {}",
			governance_authority, self.sidechain_params.genesis_committee_utxo, self.registration_utxo, self.aura_pub_key, self.grandpa_pub_key, self.sidechain_pub_key, self.sidechain_signature, spo_public_key, spo_signature));
		Ok(())
	}
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct CardanoKey {
	cbor_hex: String,
}

impl CardanoKey {
	pub fn cbor_hex_no_prefix(&self) -> &str {
		&self.cbor_hex[4..]
	}
}

fn get_mainchain_cold_skey<C: IOContext>(
	context: &C,
	keys_path: &str,
) -> Result<MainchainSigningKeyParam, anyhow::Error> {
	let cold_key = context
		.read_file(keys_path)
		.ok_or_else(|| anyhow::anyhow!("Unable to read mainchain signing key file"))?;
	let mc_signing_key = serde_json::from_str::<CardanoKey>(&cold_key)?;
	Ok(MainchainSigningKeyParam::from_str(mc_signing_key.cbor_hex_no_prefix())?)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tests::{MockIO, MockIOContext};

	#[test]
	fn happy_path() {
		let mock_context = MockIOContext::new()
			.with_json_file("/path/to/cold.skey", coldkey_content())
			.with_expected_io(
				vec![intro_msg_io(), prompt_mc_cold_key_path_io(), output_result_io()]
					.into_iter()
					.flatten()
					.collect::<Vec<MockIO>>(),
			);

		let result = mock_register2_cmd().run(&mock_context);
		result.expect("should succeed");
	}

	#[test]
	fn invalid_mc_signing_key() {
		let mock_context = MockIOContext::new().with_expected_io(vec![
			MockIO::Group(intro_msg_io()),
			MockIO::prompt(
				"Path to mainchain signing key file",
				Some("cold.skey"),
				"/invalid/cold.skey",
			),
			MockIO::file_read("/invalid/cold.skey"),
			MockIO::eprint("Unable to read mainchain signing key file"),
		]);

		let result = mock_register2_cmd().run(&mock_context);
		result.expect_err("should return error");
	}

	fn intro_msg_io() -> Vec<MockIO> {
		vec![
            MockIO::print("⚙️ Register as a committee candidate (step 2/3)"),
            MockIO::print("  This command will use SPO cold signing key for signing the registration message."),
        ]
	}

	fn prompt_mc_cold_key_path_io() -> Vec<MockIO> {
		vec![
			MockIO::prompt(
				"Path to mainchain signing key file",
				Some("cold.skey"),
				"/path/to/cold.skey",
			),
			MockIO::file_read("/path/to/cold.skey"),
		]
	}

	fn output_result_io() -> Vec<MockIO> {
		vec![
            MockIO::print("To finish the registration process, run the following command on the machine with the partner chain dependencies running:\n"),
            MockIO::print("./partner-chains-cli register3 \\\n--governance-authority 0x00112233445566778899001122334455667788990011223344556677 \\\n--genesis-committee-utxo 0000000000000000000000000000000000000000000000000000000000000001#0 \\\n--registration-utxo 7e9ebd0950ae1bec5606f0cd7ac88b3c60b1103d7feb6ffa36402edae4d1b617#0 \\\n--aura-pub-key 0xdf883ee0648f33b6103017b61be702017742d501b8fe73b1d69ca0157460b777 \\\n--grandpa-pub-key 0x5a091a06abd64f245db11d2987b03218c6bd83d64c262fe10e3a2a1230e90327 \\\n--sidechain-pub-key 0x031e75acbf45ef8df98bbe24b19b28fff807be32bf88838c30c0564d7bec5301f6 \\\n--sidechain-signature 7a7e3e585a5dc248d4a2772814e1b58c90313443dd99369f994e960ecc4931442a08305743db7ab42ab9b8672e00250e1cc7c08bc018b0630a8197c4f95528a301 \\\n--spo-public-key cef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7 \\\n--spo-signature 448ddd2592a681ee3235aa68356290c3ec93cc1b8b757bf4713a0b6629a3b75028e984a06cd275a99f861f8303dba1778c36feef084ea4a5379775ca13043202"),
        ]
	}

	fn mock_register2_cmd() -> Register2Cmd {
		Register2Cmd {
            sidechain_params: chain_params::SidechainParams {
                genesis_committee_utxo: "0000000000000000000000000000000000000000000000000000000000000001#0".parse().unwrap(),
                governance_authority: "0x00112233445566778899001122334455667788990011223344556677".parse().unwrap(),
            },
            registration_utxo: "7e9ebd0950ae1bec5606f0cd7ac88b3c60b1103d7feb6ffa36402edae4d1b617#0".parse().unwrap(),
            sidechain_pub_key: "0x031e75acbf45ef8df98bbe24b19b28fff807be32bf88838c30c0564d7bec5301f6".parse().unwrap(),
            aura_pub_key: "0xdf883ee0648f33b6103017b61be702017742d501b8fe73b1d69ca0157460b777".parse().unwrap(),
            grandpa_pub_key: "0x5a091a06abd64f245db11d2987b03218c6bd83d64c262fe10e3a2a1230e90327".parse().unwrap(),
            sidechain_signature: "7a7e3e585a5dc248d4a2772814e1b58c90313443dd99369f994e960ecc4931442a08305743db7ab42ab9b8672e00250e1cc7c08bc018b0630a8197c4f95528a301".parse().unwrap()
        }
	}

	fn coldkey_content() -> serde_json::Value {
		serde_json::json!({
			"type": "StakePoolSigningKey_ed25519",
			"description": "Stake Pool Operator Signing Key",
			"cborHex": "58200c049bb92212b779ee8ba9550536d8103cc1892634f0d21dcaa8944f5e4bf718"
		})
	}
}
