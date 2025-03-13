use sidechain_domain::{EpochNonce, ScEpochNumber};
use sp_core::U256;

pub fn seed_from_nonce_and_sc_epoch(
	epoch_nonce: &EpochNonce,
	sidechain_epoch_number: &ScEpochNumber,
) -> [u8; 32] {
	let mut epoch_nonce = epoch_nonce.0.clone();
	epoch_nonce.resize_with(32, || 0);
	let epoch_nonce: [u8; 32] =
		epoch_nonce.try_into().expect("Should never fail after being resized");
	let seed_u256: U256 = U256::from_big_endian(&epoch_nonce)
		.overflowing_add(U256::from(sidechain_epoch_number.0))
		.0;
	seed_u256.to_big_endian()
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
