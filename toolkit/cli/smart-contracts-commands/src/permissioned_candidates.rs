use crate::parse_partnerchain_public_keys;
use crate::PaymentFilePath;
use ogmios_client::jsonrpsee::client_for_url;
use partner_chains_cardano_offchain::await_tx::FixedDelayRetries;
use partner_chains_cardano_offchain::permissioned_candidates::upsert_permissioned_candidates;
use sidechain_domain::AuraPublicKey;
use sidechain_domain::GrandpaPublicKey;
use sidechain_domain::PermissionedCandidateData;
use sidechain_domain::SidechainPublicKey;
use sidechain_domain::UtxoId;
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
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
}

impl UpsertPermissionedCandidatesCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;

		let mut permissioned_candidates = Vec::new();

		for line in read_to_string(&self.permissioned_candidates_file).unwrap().lines() {
			let permissioned_candidate = parse_partnerchain_public_keys(line)
				.map_err(|e| format!("Failed to parse permissioned candidate: {}", e))?;
			permissioned_candidates.push(permissioned_candidate);
		}

		let client = client_for_url(&self.common_arguments.ogmios_url).await?;

		upsert_permissioned_candidates(
			self.genesis_utxo,
			&permissioned_candidates,
			payment_key.0,
			&client,
			&FixedDelayRetries::two_minutes(),
		)
		.await?;

		Ok(())
	}
}