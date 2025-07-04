use alloc::string::{String, ToString};
use core::time::Duration;
use serde::{Deserialize, Serialize};
use sp_runtime::Permill;

/// Configuration structure for partner chains parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartnerChainsConfig {
	/// Security parameter (k) - number of blocks for finality
	#[serde(rename = "securityParam")]
	pub security_param: u64,

	/// Active slots coefficient (f) - fraction of slots that should have blocks
	#[serde(rename = "activeSlotsCoeff")]
	pub active_slots_coeff: f64,

	/// Epoch length in slots
	#[serde(rename = "epochLength")]
	pub epoch_length: u64,

	/// Slot length in milliseconds
	#[serde(rename = "slotLength")]
	pub slot_length: u64,
}

impl Default for PartnerChainsConfig {
	fn default() -> Self {
		Self {
			security_param: 432,
			active_slots_coeff: 0.05,
			epoch_length: 432000,
			slot_length: 1000,
		}
	}
}

impl PartnerChainsConfig {
	/// Convert active slots coefficient to Permill
	pub fn active_slots_coeff_permill(&self) -> Permill {
		// Convert float to parts per million (0.05 = 50000 parts per million)
		let parts = (self.active_slots_coeff * 1_000_000.0) as u32;
		Permill::from_parts(parts)
	}

	/// Convert epoch length to Duration (in milliseconds)
	pub fn epoch_duration(&self) -> Duration {
		Duration::from_millis(self.epoch_length * self.slot_length)
	}

	/// Convert slot length to Duration
	pub fn slot_duration(&self) -> Duration {
		Duration::from_millis(self.slot_length)
	}

	/// Validate configuration parameters
	pub fn validate(&self) -> Result<(), String> {
		if self.security_param == 0 {
			return Err("Security parameter must be greater than 0".to_string());
		}

		if self.active_slots_coeff <= 0.0 || self.active_slots_coeff > 1.0 {
			return Err("Active slots coefficient must be between 0 and 1".to_string());
		}

		if self.epoch_length == 0 {
			return Err("Epoch length must be greater than 0".to_string());
		}

		if self.slot_length == 0 {
			return Err("Slot length must be greater than 0".to_string());
		}

		Ok(())
	}
}

#[cfg(feature = "std")]
impl PartnerChainsConfig {
	/// Load configuration from file path
	pub fn from_file(path: &str) -> Result<Self, String> {
		use std::fs;

		let content = fs::read_to_string(path)
			.map_err(|e| format!("Failed to read config file '{}': {}", path, e))?;

		let config: Self = serde_json::from_str(&content)
			.map_err(|e| format!("Failed to parse config file '{}': {}", path, e))?;

		config.validate()?;

		Ok(config)
	}

	/// Load configuration from environment variable or use defaults
	pub fn from_env_or_default() -> Self {
		match std::env::var("PARTNER_CHAINS_CONFIG_PATH") {
			Ok(path) => match Self::from_file(&path) {
				Ok(config) => {
					println!("Loaded partner chains configuration from: {}", path);
					config
				},
				Err(e) => {
					println!("Warning: {}", e);
					println!("Using default configuration values");
					Self::default()
				},
			},
			Err(_) => {
				println!("PARTNER_CHAINS_CONFIG_PATH not set, using default configuration");
				Self::default()
			},
		}
	}
}
