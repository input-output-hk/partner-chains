use crate::IOContext;
use anyhow::anyhow;
use partner_chains_cardano_offchain::cardano_keys::{
	CardanoKeyFileContent, CardanoPaymentSigningKey,
};
use sidechain_domain::MainchainPublicKey;

fn parse_json_key_file(
	path: &str,
	context: &impl IOContext,
) -> anyhow::Result<CardanoKeyFileContent> {
	let Some(file_content) = context.read_file(path) else {
		return Err(anyhow!("Failed to read Cardano key file {path}"));
	};
	serde_json::from_str::<CardanoKeyFileContent>(&file_content)
		.map_err(|err| anyhow!("Failed to parse Cardano key file {path}: '{err}'"))
}

pub(crate) fn get_mc_payment_signing_key_from_file(
	path: &str,
	context: &impl IOContext,
) -> anyhow::Result<CardanoPaymentSigningKey> {
	let key_file = parse_json_key_file(path, context)?;
	CardanoPaymentSigningKey::try_from(key_file)
		.map_err(|err| anyhow!("Failed to parse Cardano Payment Signing key file {path}: '{err}'"))
}

pub(crate) fn get_mc_payment_verification_key_from_file(
	path: &str,
	context: &impl IOContext,
) -> anyhow::Result<MainchainPublicKey> {
	let key_file = parse_json_key_file(path, context)?;
	let key_type = key_file.r#type.clone();
	if key_type == "PaymentExtendedVerificationKeyShelley_ed25519_bip32" {
		let key_bytes = key_file
			.raw_key_bytes::<64>()
			.map_err(|e| anyhow!("Failed to parse key bytes in {path}. {e}"))?;
		let prefix: [u8; 32] = key_bytes[0..32].try_into().unwrap();
		Ok(MainchainPublicKey(prefix))
	} else if key_type == "PaymentVerificationKeyShelley_ed25519" {
		let key_bytes = key_file
			.raw_key_bytes()
			.map_err(|e| anyhow!("Failed to parse key bytes in {path}. {e}"))?;
		Ok(MainchainPublicKey(key_bytes))
	} else {
		Err(anyhow!(
			"Unexpected key type '{key_type}' in {path}. Expected a Payment Verification Key."
		))
	}
}

pub(crate) fn get_mc_staking_signing_key_from_file(
	path: &str,
	context: &impl IOContext,
) -> anyhow::Result<[u8; 32]> {
	let key_file = parse_json_key_file(path, context)?;
	let key_type = key_file.r#type.clone();
	if key_type == "StakePoolSigningKey_ed25519" {
		key_file
			.raw_key_bytes()
			.map_err(|err| anyhow!("Failed to parse {path}: '{err}'"))
	} else {
		Err(anyhow!("Expected Stake Pool Signing Key, but got '{key_type}' in {path}"))
	}
}

pub(crate) fn get_mc_staking_verification_key_from_file(
	path: &str,
	context: &impl IOContext,
) -> anyhow::Result<MainchainPublicKey> {
	let key_file = parse_json_key_file(path, context)?;
	let key_type = key_file.r#type.clone();
	if key_type == "StakePoolVerificationKey_ed25519" {
		Ok(MainchainPublicKey(
			key_file.raw_key_bytes().map_err(|e| anyhow!("Failed to parse {path}: '{e}'"))?,
		))
	} else {
		Err(anyhow!("Expected Stake Pool Verification Key, but got '{key_type}' in {path}"))
	}
}
