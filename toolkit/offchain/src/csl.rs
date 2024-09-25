#![allow(dead_code)]

use cardano_serialization_lib::{
	Address, Credential, EnterpriseAddress, LanguageKind, NetworkIdKind,
};

pub(crate) fn plutus_script_address(
	script_bytes: &[u8],
	network: NetworkIdKind,
	language: LanguageKind,
) -> Address {
	// Before hashing the script, we need to prepend with byte 0x02, because this is PlutusV2 script
	let mut buf: Vec<u8> = vec![language_kind_to_u8(language)];
	buf.extend(script_bytes);
	let script_hash = sidechain_domain::crypto::blake2b(buf.as_slice());
	EnterpriseAddress::new(
		network_id_kind_to_u8(network),
		&Credential::from_scripthash(&script_hash.into()),
	)
	.to_address()
}

fn network_id_kind_to_u8(network: NetworkIdKind) -> u8 {
	match network {
		NetworkIdKind::Mainnet => 1,
		NetworkIdKind::Testnet => 0,
	}
}

fn language_kind_to_u8(language: LanguageKind) -> u8 {
	match language {
		LanguageKind::PlutusV1 => 1,
		LanguageKind::PlutusV2 => 2,
		LanguageKind::PlutusV3 => 3,
	}
}

#[cfg(test)]
mod tests {
	use crate::csl::plutus_script_address;
	use cardano_serialization_lib::{LanguageKind, NetworkIdKind};

	#[test]
	fn candidates_script_address_test() {
		let address = plutus_script_address(
			&crate::untyped_plutus::tests::CANDIDATES_SCRIPT_WITH_APPLIED_PARAMS,
			NetworkIdKind::Testnet,
			LanguageKind::PlutusV2,
		);
		assert_eq!(
			address.to_bech32(None).unwrap(),
			"addr_test1wq7vcwawqa29a5a2z7q8qs6k0cuvp6z2puvd8xx7vasuajq86paxz"
		);
	}
}
