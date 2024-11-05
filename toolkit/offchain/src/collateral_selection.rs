use ogmios_client::types::*;

pub const MIN_REQUIRED_COLLATERAL: u64 = 5_000_000;

// Largest-First selection algorithm based on https://cips.cardano.org/cip/CIP-0002#largest-first
pub fn largest_first(
	inputs_available: &[OgmiosUtxo],
	amount_required: u64,
	max_collateral_inputs: u32,
	min_required_collateral: u64,
) -> Result<Vec<&OgmiosUtxo>, &str> {
	let amount_required = amount_required.max(min_required_collateral);
	let amount_available: u64 = inputs_available.iter().map(|utxo| utxo.value.lovelace).sum();
	if amount_available < amount_required {
		return Err("InputValueInsufficientError");
	}
	let mut inputs_available_descending = inputs_available.iter().collect::<Vec<_>>();
	inputs_available_descending
		.sort_by(|utxo_a, utxo_b| utxo_a.value.lovelace.cmp(&utxo_b.value.lovelace).reverse());

	let inputs_selected = inputs_available_descending
		.iter()
		.scan((0, false), |(sum, is_enough), utxo| {
			*sum += utxo.value.lovelace;
			if *is_enough {
				return None;
			}
			if *sum >= amount_required {
				*is_enough = true;
			}
			Some(*utxo)
		})
		.collect::<Vec<_>>();

	if inputs_selected.len() as u32 > max_collateral_inputs {
		return Err("InputLimitExceededError");
	}
	Ok(inputs_selected)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fails_if_not_enough_utxos() {
		assert_eq!(largest_first(&utxos(), 111, 3, 10), Err("InputValueInsufficientError"));
	}

	#[test]
	fn fails_if_too_many_selected() {
		assert_eq!(largest_first(&utxos(), 109, 3, 10), Err("InputLimitExceededError"));
	}

	#[test]
	fn succeeds() {
		assert_eq!(largest_first(&utxos(), 55, 3, 10), Ok(vec![&mk_utxo(50), &mk_utxo(30)]));
	}

	fn mk_utxo(value: u64) -> OgmiosUtxo {
		OgmiosUtxo {
			transaction: OgmiosTx {
				id: hex_literal::hex!(
					"0000000000000000000000000000000000000000000000000000000000000000"
				),
			},
			index: 0,
			address: "address".to_string(),
			value: OgmiosValue::new_lovelace(value),
			datum: None,
			datum_hash: None,
			script: None,
		}
	}

	fn utxos() -> Vec<OgmiosUtxo> {
		vec![mk_utxo(10), mk_utxo(30), mk_utxo(20), mk_utxo(50)]
	}
}
