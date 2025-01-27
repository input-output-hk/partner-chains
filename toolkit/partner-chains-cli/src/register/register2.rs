use crate::cardano_key::get_mc_staking_signing_key_from_file;
use crate::io::IOContext;
use crate::CmdRun;
use clap::Parser;
use cli_commands::key_params::{
	MainchainSigningKeyParam, PlainPublicKeyParam, SidechainPublicKeyParam,
};
use cli_commands::registration_signatures::RegisterValidatorMessage;
use cli_commands::signing::mainchain_public_key_and_signature;
use sidechain_domain::UtxoId;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Register2Cmd {
	#[arg(long)]
	pub genesis_utxo: UtxoId,
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
		let mainchain_signing_key =
			get_mainchain_cold_skey(context, &mainchain_signing_key_path)
				.inspect_err(|_| context.eprint("Unable to read mainchain signing key file"))?;

		let registration_message = RegisterValidatorMessage {
			genesis_utxo: self.genesis_utxo,
			sidechain_pub_key: self.sidechain_pub_key.0.clone(),
			registration_utxo: self.registration_utxo,
		};

		let (verification_key, mc_signature) =
			mainchain_public_key_and_signature(mainchain_signing_key.0, registration_message);

		let spo_public_key = hex::encode(verification_key);
		let spo_signature = hex::encode(mc_signature.to_vec());
		let executable = context.current_executable()?;
		context.print("To finish the registration process, run the following command on the machine with the partner chain dependencies running:\n");
		context.print(&format!(
			"{executable} wizards register3 \\\n--genesis-utxo {} \\\n--registration-utxo {} \\\n--aura-pub-key {} \\\n--grandpa-pub-key {} \\\n--partner-chain-pub-key {} \\\n--partner-chain-signature {} \\\n--spo-public-key {} \\\n--spo-signature {}",
			self.genesis_utxo, self.registration_utxo, self.aura_pub_key, self.grandpa_pub_key, self.sidechain_pub_key, self.sidechain_signature, spo_public_key, spo_signature));
		Ok(())
	}
}

fn get_mainchain_cold_skey<C: IOContext>(
	context: &C,
	keys_path: &str,
) -> Result<MainchainSigningKeyParam, anyhow::Error> {
	Ok(MainchainSigningKeyParam::from(get_mc_staking_signing_key_from_file(keys_path, context)?))
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
            MockIO::print("<mock executable> wizards register3 \\\n--genesis-utxo 0000000000000000000000000000000000000000000000000000000000000001#0 \\\n--registration-utxo 7e9ebd0950ae1bec5606f0cd7ac88b3c60b1103d7feb6ffa36402edae4d1b617#0 \\\n--aura-pub-key 0xdf883ee0648f33b6103017b61be702017742d501b8fe73b1d69ca0157460b777 \\\n--grandpa-pub-key 0x5a091a06abd64f245db11d2987b03218c6bd83d64c262fe10e3a2a1230e90327 \\\n--partner-chain-pub-key 0x031e75acbf45ef8df98bbe24b19b28fff807be32bf88838c30c0564d7bec5301f6 \\\n--partner-chain-signature 7a7e3e585a5dc248d4a2772814e1b58c90313443dd99369f994e960ecc4931442a08305743db7ab42ab9b8672e00250e1cc7c08bc018b0630a8197c4f95528a301 \\\n--spo-public-key cef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7 \\\n--spo-signature aaa39fbf163ed77c69820536f5dc22854e7e13f964f1e077efde0844a09bde64c1aab4d2b401e0fe39b43c91aa931cad26fa55c8766378462c06d86c85134801"),
        ]
	}
	// non 0 genesis utxo 8ea10040249ad3033ae7c4d4b69e0b2e2b50a90741b783491cb5ddf8ced0d861
	fn mock_register2_cmd() -> Register2Cmd {
		Register2Cmd {
            genesis_utxo: "0000000000000000000000000000000000000000000000000000000000000001#0".parse().unwrap(),
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
