use crate::pc_contracts_cli_resources::PcContractsCliResources;
use sidechain_domain::UtxoId;

pub fn sidechain_params_arguments(genesis_utxo: UtxoId) -> String {
	format!("--genesis-utxo {}", genesis_utxo)
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
