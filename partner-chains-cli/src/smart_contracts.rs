use crate::config::SidechainParams;
use crate::pc_contracts_cli_resources::PcContractsCliResources;

pub fn sidechain_params_arguments(sidechain_params: &SidechainParams) -> String {
	format!("--sidechain-id {} --genesis-committee-hash-utxo {} --threshold-numerator {} --threshold-denominator {} --governance-authority {} --atms-kind plain-ecdsa-secp256k1",
			sidechain_params.chain_id,
			sidechain_params.genesis_committee_utxo,
			sidechain_params.threshold_numerator,
			sidechain_params.threshold_denominator,
			sidechain_params.governance_authority.to_hex_string())
}

pub fn runtime_config_arguments(
	runtime_config: &PcContractsCliResources,
	payment_signing_key_path: &str,
) -> String {
	format!("--kupo-host {} --kupo-port {} {} --ogmios-host {} --ogmios-port {} {} --payment-signing-key-file {}",
			runtime_config.kupo.hostname,
			runtime_config.kupo.port,
			if runtime_config.kupo.protocol.is_secure() { "--kupo-secure" } else { "" },
			runtime_config.ogmios.hostname,
			runtime_config.ogmios.port,
			if runtime_config.ogmios.protocol.is_secure() { "--ogmios-secure" } else { "" },
			payment_signing_key_path
	)
}
