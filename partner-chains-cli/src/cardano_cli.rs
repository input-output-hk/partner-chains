pub fn testnet_magic_arg(cardano_network: u32) -> String {
	if cardano_network == 0 {
		"--mainnet".to_string()
	} else {
		format!("--testnet-magic {cardano_network}")
	}
}
