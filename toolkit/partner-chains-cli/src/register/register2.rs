use super::{CandidateKeyParam, RegisterValidatorMessage};
use super::{PartnerChainPublicKeyParam, StakePoolSigningKeyParam};
use crate::CmdRun;
use crate::cardano_key::get_mc_staking_signing_key_from_file;
use crate::io::IOContext;
use clap::Parser;
use sidechain_domain::UtxoId;
use sidechain_domain::crypto::cardano_spo_public_key_and_signature;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Register2Cmd {
	#[arg(long)]
	pub genesis_utxo: UtxoId,
	#[arg(long)]
	pub registration_utxo: UtxoId,
	#[arg(long)]
	pub partner_chain_pub_key: PartnerChainPublicKeyParam,
	#[arg(long)]
	pub keys: Vec<CandidateKeyParam>,
	#[arg(long)]
	pub partner_chain_signature: String,
}

impl CmdRun for Register2Cmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		let Register2Cmd {
			genesis_utxo,
			registration_utxo,
			partner_chain_pub_key,
			keys,
			partner_chain_signature,
		} = self;

		context.print("⚙️ Register as a committee candidate (step 2/3)");
		context.print(
			"  This command will use SPO cold signing key for signing the registration message.",
		);

		let stake_pool_signing_key_path =
			context.prompt("Path to Stake Pool signing key file", Some("cold.skey"));
		let mainchain_signing_key = get_stake_pool_cold_skey(context, &stake_pool_signing_key_path)
			.inspect_err(|_| context.eprint("Unable to read Stake Pool signing key file"))?;

		let registration_message = RegisterValidatorMessage {
			genesis_utxo: self.genesis_utxo,
			sidechain_pub_key: self.partner_chain_pub_key.0.clone(),
			registration_utxo: self.registration_utxo,
		};

		let (verification_key, spo_signature) =
			cardano_spo_public_key_and_signature(mainchain_signing_key.0, registration_message);

		let spo_public_key = verification_key.to_hex_string();
		let spo_signature = spo_signature.to_hex_string();
		let executable = context.current_executable()?;
		context.print("To finish the registration process, run the following command on the machine with the partner chain dependencies running:\n");
		context.print(&format!(
			"{executable} wizards register3 \\
--genesis-utxo {genesis_utxo} \\
--registration-utxo {registration_utxo} \\
--partner-chain-pub-key {partner_chain_pub_key} \\
--partner-chain-signature {partner_chain_signature} \\
--spo-public-key {spo_public_key} \\
--spo-signature {spo_signature}{}",
			keys.iter()
				.map(CandidateKeyParam::to_string)
				.map(|arg| format!(" \\\n--keys {arg}"))
				.collect::<Vec<_>>()
				.join("")
		));
		Ok(())
	}
}

fn get_stake_pool_cold_skey<C: IOContext>(
	context: &C,
	keys_path: &str,
) -> Result<StakePoolSigningKeyParam, anyhow::Error> {
	Ok(StakePoolSigningKeyParam::from(get_mc_staking_signing_key_from_file(keys_path, context)?))
}

#[cfg(test)]
mod tests {
	use hex_literal::hex;

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
	fn invalid_stake_pool_signing_key() {
		let mock_context = MockIOContext::new().with_expected_io(vec![
			MockIO::Group(intro_msg_io()),
			MockIO::prompt(
				"Path to Stake Pool signing key file",
				Some("cold.skey"),
				"/invalid/cold.skey",
			),
			MockIO::eprint("Unable to read Stake Pool signing key file"),
		]);

		let result = mock_register2_cmd().run(&mock_context);
		result.expect_err("should return error");
	}

	fn intro_msg_io() -> Vec<MockIO> {
		vec![
			MockIO::print("⚙️ Register as a committee candidate (step 2/3)"),
			MockIO::print(
				"  This command will use SPO cold signing key for signing the registration message.",
			),
		]
	}

	fn prompt_mc_cold_key_path_io() -> Vec<MockIO> {
		vec![MockIO::prompt(
			"Path to Stake Pool signing key file",
			Some("cold.skey"),
			"/path/to/cold.skey",
		)]
	}

	fn output_result_io() -> Vec<MockIO> {
		vec![
			MockIO::print(
				"To finish the registration process, run the following command on the machine with the partner chain dependencies running:\n",
			),
			MockIO::print(
				"<mock executable> wizards register3 \\
--genesis-utxo 0000000000000000000000000000000000000000000000000000000000000001#0 \\
--registration-utxo 7e9ebd0950ae1bec5606f0cd7ac88b3c60b1103d7feb6ffa36402edae4d1b617#0 \\
--partner-chain-pub-key 0x031e75acbf45ef8df98bbe24b19b28fff807be32bf88838c30c0564d7bec5301f6 \\
--partner-chain-signature 7a7e3e585a5dc248d4a2772814e1b58c90313443dd99369f994e960ecc4931442a08305743db7ab42ab9b8672e00250e1cc7c08bc018b0630a8197c4f95528a301 \\
--spo-public-key 0xcef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7 \\
--spo-signature 0xaaa39fbf163ed77c69820536f5dc22854e7e13f964f1e077efde0844a09bde64c1aab4d2b401e0fe39b43c91aa931cad26fa55c8766378462c06d86c85134801 \\
--keys aura:df883ee0648f33b6103017b61be702017742d501b8fe73b1d69ca0157460b777 \\
--keys gran:5a091a06abd64f245db11d2987b03218c6bd83d64c262fe10e3a2a1230e90327",
			),
		]
	}
	// non 0 genesis utxo 8ea10040249ad3033ae7c4d4b69e0b2e2b50a90741b783491cb5ddf8ced0d861
	fn mock_register2_cmd() -> Register2Cmd {
		Register2Cmd {
            genesis_utxo: "0000000000000000000000000000000000000000000000000000000000000001#0".parse().unwrap(),
            registration_utxo: "7e9ebd0950ae1bec5606f0cd7ac88b3c60b1103d7feb6ffa36402edae4d1b617#0".parse().unwrap(),
            partner_chain_pub_key: "0x031e75acbf45ef8df98bbe24b19b28fff807be32bf88838c30c0564d7bec5301f6".parse().unwrap(),
			keys: vec![
				CandidateKeyParam::new(*b"aura", hex!("df883ee0648f33b6103017b61be702017742d501b8fe73b1d69ca0157460b777").to_vec()),
				CandidateKeyParam::new(*b"gran", hex!("5a091a06abd64f245db11d2987b03218c6bd83d64c262fe10e3a2a1230e90327").to_vec())
			],
            partner_chain_signature: "7a7e3e585a5dc248d4a2772814e1b58c90313443dd99369f994e960ecc4931442a08305743db7ab42ab9b8672e00250e1cc7c08bc018b0630a8197c4f95528a301".parse().unwrap()
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
