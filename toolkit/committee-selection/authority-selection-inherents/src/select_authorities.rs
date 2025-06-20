//! Functionality related to selecting the validators from the valid candidates

use crate::authority_selection_inputs::AuthoritySelectionInputs;
use crate::filter_invalid_candidates::{
	Candidate, filter_invalid_permissioned_candidates, filter_trustless_candidates_registrations,
};
use log::{info, warn};
use plutus::*;
use schnorr_jubjub;
use sidechain_domain::{EpochNonce, ScEpochNumber, UtxoId};
use sp_core::{U256, ecdsa, ed25519, sr25519};

/// Selects authorities using the Ariadne selection algorithm and data sourced from Partner Chains smart contracts on Cardano.
/// Seed is constructed from the MC epoch nonce and the sidechain epoch.
pub fn select_authorities<
	TAccountId: Clone + Ord + TryFrom<sidechain_domain::SidechainPublicKey> + From<ecdsa::Public>,
	TAccountKeys: Clone + Ord + From<(sr25519::Public, ed25519::Public, ed25519::Public)>,
>(
	genesis_utxo: UtxoId,
	input: AuthoritySelectionInputs,
	sidechain_epoch: ScEpochNumber,
) -> Option<Vec<Candidate<TAccountId, TAccountKeys>>> {
	let valid_registered_candidates = filter_trustless_candidates_registrations::<
		TAccountId,
		TAccountKeys,
	>(input.registered_candidates, genesis_utxo);
	let valid_permissioned_candidates =
		filter_invalid_permissioned_candidates(input.permissioned_candidates);
	let valid_permissioned_count = valid_permissioned_candidates.len();
	let valid_registered_count = valid_registered_candidates.len();

	let random_seed = seed_from_nonce_and_sc_epoch(&input.epoch_nonce, &sidechain_epoch);

	if let Some(validators) = selection::ariadne_v2::select_authorities(
		input.d_parameter.num_registered_candidates,
		input.d_parameter.num_permissioned_candidates,
		valid_registered_candidates,
		valid_permissioned_candidates,
		random_seed,
	) {
		info!(
			"💼 Selected committee of {} seats for epoch {} from {valid_permissioned_count} permissioned and {valid_registered_count} registered candidates",
			validators.len(),
			sidechain_epoch
		);
		Some(validators)
	} else {
		warn!("🚫 Failed to select validators for epoch {}", sidechain_epoch);
		None
	}
}

/// Generate 32 byte seed from epoch nonce and Partner Chain epoch number
pub fn seed_from_nonce_and_sc_epoch(
	epoch_nonce: &EpochNonce,
	partner_chain_epoch_number: &ScEpochNumber,
) -> [u8; 32] {
	U256::from_big_endian(&epoch_nonce.as_array())
		.overflowing_add(U256::from(partner_chain_epoch_number.0))
		.0
		.to_big_endian()
}

#[cfg(test)]
mod tests {
	use super::*;
	use sidechain_domain::{EpochNonce, ScEpochNumber};
	use sp_core::U256;

	#[test]
	fn should_create_correct_seed() {
		let nonce_vec = Vec::from(U256::from(10).to_big_endian());
		assert_eq!(
			seed_from_nonce_and_sc_epoch(&EpochNonce(nonce_vec), &ScEpochNumber(2)),
			U256::from(12).to_big_endian()
		);
	}
}
