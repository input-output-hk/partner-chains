use crate::BlockProducerIdInherentProvider;
use sealed_test::prelude::*;

const BLOCK_PRODUCER_ID_ENV_VAR_VALUE: &str =
	"020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9";

#[sealed_test]
fn from_env_happy_path() {
	std::env::set_var("TEST_VAR_NAME", BLOCK_PRODUCER_ID_ENV_VAR_VALUE);
	let provider = BlockProducerIdInherentProvider::<[u8; 32]>::from_env("TEST_VAR_NAME").unwrap();
	assert_eq!(hex::encode(provider.id), BLOCK_PRODUCER_ID_ENV_VAR_VALUE)
}

#[sealed_test]
fn from_env_happy_path_with_0x_prefix() {
	let env_var_value = format!("0x{}", BLOCK_PRODUCER_ID_ENV_VAR_VALUE);
	std::env::set_var("TEST_VAR_NAME", env_var_value);
	let provider = BlockProducerIdInherentProvider::<[u8; 32]>::from_env("TEST_VAR_NAME").unwrap();
	assert_eq!(hex::encode(provider.id), BLOCK_PRODUCER_ID_ENV_VAR_VALUE)
}

#[sealed_test]
fn error_when_env_var_is_not_set() {
	std::env::remove_var("TEST_VAR_NAME");
	let err = BlockProducerIdInherentProvider::<[u8; 32]>::from_env("TEST_VAR_NAME").unwrap_err();
	assert_eq!(err, "Block Producer Id environment variable 'TEST_VAR_NAME' is not set");
}

#[sealed_test]
fn error_when_invalid_hex() {
	std::env::set_var("TEST_VAR_NAME", "0xabcdzzzzz");
	let err = BlockProducerIdInherentProvider::<[u8; 32]>::from_env("TEST_VAR_NAME").unwrap_err();
	assert_eq!(err, "Block Producer Id environment variable 'TEST_VAR_NAME' value '0xabcdzzzzz' is not a valid hex string");
}

#[sealed_test]
fn error_when_failed_try_from() {
	std::env::set_var("TEST_VAR_NAME", BLOCK_PRODUCER_ID_ENV_VAR_VALUE);
	// Value is 32 bytes, expected 33
	let err = BlockProducerIdInherentProvider::<[u8; 33]>::from_env("TEST_VAR_NAME").unwrap_err();
	assert!(err.starts_with("Could not convert '020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9' into Block Producer Id. Cause:"));
}
