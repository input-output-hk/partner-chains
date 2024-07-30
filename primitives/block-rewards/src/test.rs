use crate::BlockBeneficiaryInherentProvider;
use sealed_test::prelude::*;
use sp_core::bytes::from_hex;

const BENEFICIARY_ID: &str = "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9";

#[sealed_test]
fn inherent_data_is_provided_from_env() {
	std::env::set_var("ENV_VAR", BENEFICIARY_ID);
	let provider = BlockBeneficiaryInherentProvider::<[u8; 32]>::from_env("ENV_VAR")
		.expect("Provider should be created");

	assert_eq!(provider.beneficiary_id.to_vec(), from_hex(BENEFICIARY_ID).unwrap())
}

#[sealed_test]
fn error_when_no_env_var() {
	std::env::remove_var("ENV_VAR");
	let provider = BlockBeneficiaryInherentProvider::<[u8; 32]>::from_env("ENV_VAR");

	assert!(provider.is_err())
}

#[sealed_test]
fn error_when_invalid_hex() {
	std::env::set_var("ENV_VAR", "0xabcdzzzzz");
	let provider = BlockBeneficiaryInherentProvider::<[u8; 32]>::from_env("ENV_VAR");

	assert!(provider.is_err())
}

#[sealed_test]
fn error_when_failed_try_from() {
	std::env::set_var("ENV_VAR", BENEFICIARY_ID);
	// BENEFICIARY_ID is 32 bytes, expected 33
	let provider = BlockBeneficiaryInherentProvider::<[u8; 33]>::from_env("ENV_VAR");

	assert!(provider.is_err())
}
