use crate::{
	config::ServiceConfig,
	ogmios::{OgmiosRequest, OgmiosResponse},
	IOContext,
};
use anyhow::anyhow;
use ogmios_client::types::OgmiosUtxo;
use sidechain_domain::{McTxHash, UtxoId};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub(crate) struct ValidUtxo {
	pub utxo_id: UtxoId,
	pub lovelace: u64,
}

impl ValidUtxo {
	pub fn to_display_string(&self) -> String {
		format!("{0} ({1} lovelace)", self.utxo_id, self.lovelace)
	}
}

pub(crate) fn query_utxos<C: IOContext>(
	context: &C,
	ogmios_config: &ServiceConfig,
	address: &str,
) -> Result<Vec<OgmiosUtxo>, anyhow::Error> {
	let ogmios_addr = ogmios_config.to_string();
	context.print(&format!("⚙️ Querying UTXOs of {address} from Ogmios at {ogmios_addr}..."));
	let response = context
		.ogmios_rpc(&ogmios_addr, OgmiosRequest::QueryUtxo { address: address.into() })
		.map_err(|e| anyhow!(e))?;
	match response {
		OgmiosResponse::QueryUtxo(utxos) => Ok(utxos),
		other => Err(anyhow::anyhow!(format!(
			"Unexpected response from Ogmios when querying for utxos: {other:?}"
		))),
	}
}

// Take only the UTXOs without multi-asset tokens
pub(crate) fn filter_utxos(utxos: Vec<OgmiosUtxo>) -> Vec<ValidUtxo> {
	let mut utxos: Vec<ValidUtxo> = utxos
		.into_iter()
		.filter_map(|utxo| {
			if utxo.value.native_tokens.is_empty() {
				Some(ValidUtxo {
					utxo_id: UtxoId {
						tx_hash: McTxHash(utxo.transaction.id),
						index: sidechain_domain::UtxoIndex(utxo.index),
					},
					lovelace: utxo.value.lovelace,
				})
			} else {
				None
			}
		})
		.collect();

	utxos.sort_by_key(|utxo| std::cmp::Reverse(utxo.lovelace));
	utxos
}

pub(crate) fn select_from_utxos<C: IOContext>(
	context: &C,
	prompt: &str,
	utxos: Vec<ValidUtxo>,
) -> Result<UtxoId, anyhow::Error> {
	let utxo_display_options: Vec<String> =
		utxos.iter().map(|utxo| utxo.to_display_string()).collect();
	let selected_utxo_display_string = context.prompt_multi_option(prompt, utxo_display_options);
	let selected_utxo = utxos
		.iter()
		.find(|utxo| utxo.to_display_string() == selected_utxo_display_string)
		.map(|utxo| utxo.utxo_id.to_string())
		.ok_or_else(|| anyhow!("⚠️ Failed to find selected UTXO"))?;
	UtxoId::from_str(&selected_utxo).map_err(|e| {
		context.eprint(&format!("⚠️ Failed to parse selected UTXO: {e}"));
		anyhow!(e)
	})
}

#[cfg(test)]
pub(crate) mod tests {
	use crate::{
		ogmios::{OgmiosRequest, OgmiosResponse},
		select_utxo::{filter_utxos, ValidUtxo},
		tests::MockIO,
	};
	use hex_literal::hex;
	use ogmios_client::types::{Asset, OgmiosTx, OgmiosUtxo, OgmiosValue};
	use sidechain_domain::UtxoId;
	use std::{collections::HashMap, str::FromStr};

	#[test]
	fn test_parse_utxo_query_output() {
		{
			let utxos = filter_utxos(mock_result_5_valid());

			assert_eq!(utxos.len(), 5);
			assert_eq!(
				utxos[0],
				ValidUtxo {
					utxo_id: UtxoId::from_str(
						"f5f58c0d5ab357a3562ca043a4dd67567a8399da77968cef59fb271d72db57bd#0"
					)
					.unwrap(),
					lovelace: 1700000,
				}
			);
			assert_eq!(
				utxos[1],
				ValidUtxo {
					utxo_id: UtxoId::from_str(
						"b031cda9c257fed6eed781596ab5ca9495ae88a860e807763b2cd67c72c4cc1e#0"
					)
					.unwrap(),
					lovelace: 1500000,
				}
			);
			assert_eq!(
				utxos[2],
				ValidUtxo {
					utxo_id: UtxoId::from_str(
						"917e3dba3ed5faee7855d99b4a797859ac7b1941b381aef36080d767127bdaba#0"
					)
					.unwrap(),
					lovelace: 1400000,
				}
			);
			assert_eq!(
				utxos[3],
				ValidUtxo {
					utxo_id: UtxoId::from_str(
						"76ddb0a474eb893e6e17de4cc692bce12e57271351cccb4c0e7e2ad864347b64#0"
					)
					.unwrap(),
					lovelace: 1200000,
				}
			);
			assert_eq!(
				utxos[4],
				ValidUtxo {
					utxo_id: UtxoId::from_str(
						"4704a903b01514645067d851382efd4a6ed5d2ff07cf30a538acc78fed7c4c02#93"
					)
					.unwrap(),
					lovelace: 1100000,
				}
			);
		}

		{
			let utxos = filter_utxos(mock_result_0_valid());
			assert_eq!(utxos.len(), 0);
		}
	}

	pub(crate) fn mock_result_5_valid() -> Vec<OgmiosUtxo> {
		vec![
			OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("4704a903b01514645067d851382efd4a6ed5d2ff07cf30a538acc78fed7c4c02"),
				},
				index: 93,
				value: OgmiosValue::new_lovelace(1100000),
				..Default::default()
			},
			OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("76ddb0a474eb893e6e17de4cc692bce12e57271351cccb4c0e7e2ad864347b64"),
				},
				index: 0,
				value: OgmiosValue::new_lovelace(1200000),
				..Default::default()
			},
			OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("b9da3bfe0c7c177d494aeea0937ce4da9827c8dfc80bedb5825cd08887cbedb8"),
				},
				index: 0,
				value: OgmiosValue {
					lovelace: 1300000,
					native_tokens: HashMap::from([(
						hex!("244d83c5418732113e891db15ede8f0d15df75b705a1542d86937875"),
						vec![Asset {
							name: hex!("4c757854657374546f6b656e54727932").to_vec(),
							amount: 1,
						}],
					)]),
				},
				..Default::default()
			},
			OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("917e3dba3ed5faee7855d99b4a797859ac7b1941b381aef36080d767127bdaba"),
				},
				index: 0,
				value: OgmiosValue::new_lovelace(1400000),
				..Default::default()
			},
			OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("b031cda9c257fed6eed781596ab5ca9495ae88a860e807763b2cd67c72c4cc1e"),
				},
				index: 0,
				value: OgmiosValue::new_lovelace(1500000),
				..Default::default()
			},
			OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("b9da3bfe0c7c177d494aeea0937ce4da9827c8dfc80bedb5825cd08887cbedb8"),
				},
				index: 0,
				value: OgmiosValue {
					lovelace: 1600000,
					native_tokens: HashMap::from([(
						hex!("7726c67e096e60ff24757de0ec0a78c659ce73c9b12e98df7d2fda2c"),
						vec![Asset { name: vec![], amount: 1 }],
					)]),
				},
				..Default::default()
			},
			OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("f5f58c0d5ab357a3562ca043a4dd67567a8399da77968cef59fb271d72db57bd"),
				},
				index: 0,
				value: OgmiosValue::new_lovelace(1700000),
				..Default::default()
			},
		]
	}

	pub(crate) fn mock_5_valid_utxos_rows() -> Vec<String> {
		vec![
			"f5f58c0d5ab357a3562ca043a4dd67567a8399da77968cef59fb271d72db57bd#0 (1700000 lovelace)".to_string(),
			"b031cda9c257fed6eed781596ab5ca9495ae88a860e807763b2cd67c72c4cc1e#0 (1500000 lovelace)".to_string(),
			"917e3dba3ed5faee7855d99b4a797859ac7b1941b381aef36080d767127bdaba#0 (1400000 lovelace)".to_string(),
			"76ddb0a474eb893e6e17de4cc692bce12e57271351cccb4c0e7e2ad864347b64#0 (1200000 lovelace)".to_string(),
			"4704a903b01514645067d851382efd4a6ed5d2ff07cf30a538acc78fed7c4c02#93 (1100000 lovelace)".to_string(),
		]
	}

	pub(crate) fn mock_result_0_valid() -> Vec<OgmiosUtxo> {
		vec![OgmiosUtxo {
			transaction: OgmiosTx {
				id: hex!("8a0d3e5644b3e84a775556b44e6407971d01b8bfa3f339294b7228ac18ddb29c"),
			},
			index: 0,
			value: OgmiosValue {
				lovelace: 10000000,
				native_tokens: HashMap::from([(
					hex!("244d83c5418732113e891db15ede8f0d15df75b705a1542d86937875"),
					vec![Asset {
						name: hex!("4c757854657374546f6b656e54727932").to_vec(),
						amount: 1,
					}],
				)]),
			},
			..Default::default()
		}]
	}

	pub(crate) fn query_utxos_io(
		cardano_addr: &str,
		ogmios_addr: &'static str,
		result: Vec<OgmiosUtxo>,
	) -> MockIO {
		MockIO::Group(vec![
			MockIO::print(&format!(
				"⚙️ Querying UTXOs of {} from Ogmios at {}...",
				cardano_addr, ogmios_addr
			)),
			MockIO::ogmios_request(
				ogmios_addr,
				OgmiosRequest::QueryUtxo { address: cardano_addr.into() },
				Ok(OgmiosResponse::QueryUtxo(result)),
			),
		])
	}
}
