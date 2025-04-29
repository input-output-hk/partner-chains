use crate::{GenesisUtxo, PaymentFilePath, option_to_json, parse_partnerchain_public_keys};
use partner_chains_cardano_offchain::permissioned_candidates::upsert_permissioned_candidates;
use std::fs::read_to_string;

#[derive(Clone, Debug, clap::Parser)]
pub struct UpsertPermissionedCandidatesCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	/// Path to the file containing the permissioned candidates data.
	/// Each line represents one permissioned candidate in format SIDECHAIN_KEY:AURA_KEY:GRANDPA_KEY
	#[arg(long)]
	permissioned_candidates_file: String,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	genesis_utxo: GenesisUtxo,
}

impl UpsertPermissionedCandidatesCmd {
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
