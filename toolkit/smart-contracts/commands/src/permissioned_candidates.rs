use crate::{GenesisUtxo, PaymentFilePath, option_to_json, parse_partnerchain_public_keys};
use partner_chains_cardano_offchain::permissioned_candidates::upsert_permissioned_candidates;
use std::fs::read_to_string;

#[derive(Clone, Debug, clap::Parser)]
/// Command for upserting the permissioned candidates on the main chain
pub struct UpsertPermissionedCandidatesCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	/// Path to the file containing the permissioned candidates data.
	/// Each line represents one permissioned candidate in format PARTNER_CHAINS_KEY,KEY_1_ID:KEY_1_BYTES,...,KEY_N_ID:KEY_N_BYTES.
	/// Legacy format of PARTNER_CHAINS_KEY:AURA_PUB_KEY:GRANDPA_PUB_KEY is supported, each line is eqivalent to `PARTNER_CHAINS_KEY,aura:AURA_PUB_KEY,gran:GRANDPA_PUB_KEY`.
	permissioned_candidates_file: String,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
}

impl UpsertPermissionedCandidatesCmd {
	/// Upserts the permissioned candidates on the main chain.
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;

		let mut permissioned_candidates = Vec::new();

		let file_content = read_to_string(&self.permissioned_candidates_file).map_err(|e| {
			format!(
				"Could not read permissioned candidates file '{}'. Cause: {e}",
				&self.permissioned_candidates_file
			)
		})?;
		for line in file_content.lines() {
			if line.is_empty() {
				continue;
			}
			let permissioned_candidate = parse_partnerchain_public_keys(line).map_err(|e| {
				format!("Failed to parse permissioned candidate: '{}', because of {}", line, e)
			})?;
			permissioned_candidates.push(permissioned_candidate);
		}

		let client = self.common_arguments.get_ogmios_client().await?;

		let result = upsert_permissioned_candidates(
			self.genesis_utxo.into(),
			&permissioned_candidates,
			&payment_key,
			&client,
			&self.common_arguments.retries(),
		)
		.await?;
		Ok(option_to_json(result))
	}
}
