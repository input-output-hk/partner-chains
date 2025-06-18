//! Tests for pallet-cardano-config

use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};
use sidechain_domain::{CardanoConfig, MainchainEpochConfig};
use sp_core::offchain::{Duration, Timestamp};

fn sample_cardano_config() -> CardanoConfig {
	CardanoConfig {
		epoch_config: MainchainEpochConfig {
			epoch_duration_millis: Duration::from_millis(432000000), // 5 days
			slot_duration_millis: Duration::from_millis(1000),       // 1 second
			first_epoch_timestamp_millis: Timestamp::from_unix_millis(1596059091000),
			first_epoch_number: 208,
			first_slot_number: 4492800,
		},
		cardano_security_parameter: 432,
		cardano_active_slots_coeff: 0.05,
	}
}

#[test]
fn genesis_config_works() {
	new_test_ext_with_config().execute_with(|| {
		let expected_config = sample_cardano_config();
		
		// Check that configuration was set at genesis
		assert!(CardanoConfig::is_configured());
		assert_eq!(CardanoConfig::get_cardano_config(), Some(expected_config.clone()));
		assert_eq!(CardanoConfig::get_mainchain_epoch_config(), Some(expected_config.epoch_config));
		assert_eq!(CardanoConfig::get_cardano_security_parameter(), Some(expected_config.cardano_security_parameter));
		assert_eq!(CardanoConfig::get_cardano_active_slots_coeff(), Some(expected_config.cardano_active_slots_coeff));
	});
}

#[test]
fn set_cardano_config_works() {
	new_test_ext().execute_with(|| {
		let config = sample_cardano_config();
		
		// Initially not configured
		assert!(!CardanoConfig::is_configured());
		assert_eq!(CardanoConfig::get_cardano_config(), None);
		
		// Set configuration
		assert_ok!(CardanoConfig::set_cardano_config(RuntimeOrigin::root(), config.clone()));
		
		// Check that configuration was set
		assert!(CardanoConfig::is_configured());
		assert_eq!(CardanoConfig::get_cardano_config(), Some(config.clone()));
		assert_eq!(CardanoConfig::get_mainchain_epoch_config(), Some(config.epoch_config));
		assert_eq!(CardanoConfig::get_cardano_security_parameter(), Some(config.cardano_security_parameter));
		assert_eq!(CardanoConfig::get_cardano_active_slots_coeff(), Some(config.cardano_active_slots_coeff));
	});
}

#[test]
fn set_cardano_config_requires_root() {
	new_test_ext().execute_with(|| {
		let config = sample_cardano_config();
		
		// Non-root origin should fail
		assert_noop!(
			CardanoConfig::set_cardano_config(RuntimeOrigin::signed(1), config),
			DispatchError::BadOrigin
		);
		
		// Configuration should not be set
		assert!(!CardanoConfig::is_configured());
	});
}

#[test]
fn set_cardano_config_only_once() {
	new_test_ext().execute_with(|| {
		let config = sample_cardano_config();
		
		// Set configuration first time
		assert_ok!(CardanoConfig::set_cardano_config(RuntimeOrigin::root(), config.clone()));
		
		// Try to set again - should fail
		assert_noop!(
			CardanoConfig::set_cardano_config(RuntimeOrigin::root(), config),
			Error::<Test>::ConfigurationAlreadySet
		);
	});
}

#[test]
fn set_cardano_config_fails_if_already_set_at_genesis() {
	new_test_ext_with_config().execute_with(|| {
		let config = sample_cardano_config();
		
		// Configuration already set at genesis
		assert!(CardanoConfig::is_configured());
		
		// Try to set again - should fail
		assert_noop!(
			CardanoConfig::set_cardano_config(RuntimeOrigin::root(), config),
			Error::<Test>::ConfigurationAlreadySet
		);
	});
}

#[test]
fn getters_work_correctly() {
	new_test_ext().execute_with(|| {
		// Initially nothing set
		assert_eq!(CardanoConfig::get_cardano_config(), None);
		assert_eq!(CardanoConfig::get_mainchain_epoch_config(), None);
		assert_eq!(CardanoConfig::get_cardano_security_parameter(), None);
		assert_eq!(CardanoConfig::get_cardano_active_slots_coeff(), None);
		assert!(!CardanoConfig::is_configured());
		
		let config = sample_cardano_config();
		assert_ok!(CardanoConfig::set_cardano_config(RuntimeOrigin::root(), config.clone()));
		
		// All getters should return correct values
		assert_eq!(CardanoConfig::get_cardano_config(), Some(config.clone()));
		assert_eq!(CardanoConfig::get_mainchain_epoch_config(), Some(config.epoch_config));
		assert_eq!(CardanoConfig::get_cardano_security_parameter(), Some(config.cardano_security_parameter));
		assert_eq!(CardanoConfig::get_cardano_active_slots_coeff(), Some(config.cardano_active_slots_coeff));
		assert!(CardanoConfig::is_configured());
	});
}

#[test]
fn version_is_correct() {
	assert_eq!(CardanoConfig::get_version(), 1);
}

#[test]
fn different_configs_work() {
	new_test_ext().execute_with(|| {
		let mut config = sample_cardano_config();
		config.cardano_security_parameter = 100;
		config.cardano_active_slots_coeff = 0.1;
		config.epoch_config.epoch_duration_millis = Duration::from_millis(86400000); // 1 day
		
		assert_ok!(CardanoConfig::set_cardano_config(RuntimeOrigin::root(), config.clone()));
		
		assert_eq!(CardanoConfig::get_cardano_config(), Some(config.clone()));
		assert_eq!(CardanoConfig::get_cardano_security_parameter(), Some(100));
		assert_eq!(CardanoConfig::get_cardano_active_slots_coeff(), Some(0.1));
		assert_eq!(CardanoConfig::get_mainchain_epoch_config(), Some(config.epoch_config));
	});
}
