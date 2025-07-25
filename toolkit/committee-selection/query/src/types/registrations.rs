//! Types for Candidates Registrations returned by RPC endpoints

use crate::types::keys_to_map;
use authority_selection_inherents::{RegistrationDataError, StakeError};
use parity_scale_codec::Decode;
use serde::{Deserialize, Serialize};
use sidechain_domain::{
	RegistrationData, SidechainPublicKey, StakeDelegation, StakePoolPublicKey, UtxoInfo,
	byte_string::ByteString,
};
use sp_core::{
	bytes::to_hex,
	crypto::{AccountId32, Ss58Codec},
	ecdsa,
};
use sp_runtime::{MultiSigner, traits::IdentifyAccount};
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, PartialEq, Eq, Clone, Decode, thiserror::Error, Serialize, Deserialize)]
/// Registration error type
pub enum RegistrationError {
	#[error("{0}")]
	/// A wrapped [StakeError]
	StakeError(#[from] StakeError),
	#[error("{0}")]
	/// A wrapped [RegistrationDataError]
	InvalidRegistrationData(#[from] RegistrationDataError),
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Candidate Registration Entry
pub struct CandidateRegistrationEntry {
	/// Sidechain public key of the candidate. See [sidechain_domain::SidechainPublicKey] for more details.
	pub sidechain_pub_key: String,
	/// SS58 address derived from public key. ss58(blake2b32(sidechainPubKey))
	pub sidechain_account_id: String,
	/// Stake Pool public key
	pub mainchain_pub_key: String,
	/// Cross chain public key of the candidate. See [sidechain_domain::CrossChainPublicKey] for more details.
	pub cross_chain_pub_key: String,
	/// All user-defined keys that are read from Cardano
	pub keys: HashMap<String, ByteString>,
	/// Sidechain key signature of the registration message
	pub sidechain_signature: String,
	/// Signature made with Stake Pool key
	pub mainchain_signature: String,
	/// Cross chain key signature of the registration message
	pub cross_chain_signature: String,
	/// Data of UTxO that contained this registration
	pub utxo: UtxoInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Total stake delegated to the pool identified by `mainchain_pub_key`.
	/// [None] if registration is not stable yet.
	pub stake_delegation: Option<u64>,
	/// Is the registration valid
	pub is_valid: bool,
	/// Human readable reasons of registration being invalid. Present only for invalid entries.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub invalid_reasons: Option<RegistrationError>,
}

/// Type mapping the candidate's mainchain pub key in hex string format to its registration entry
pub type GetRegistrationsResponseMap = HashMap<String, Vec<CandidateRegistrationEntry>>;

impl CandidateRegistrationEntry {
	/// Constructor for [CandidateRegistrationEntry]
	pub fn new(
		registration_data: RegistrationData,
		stake_pool_public_key: StakePoolPublicKey,
		stake_delegation: Option<StakeDelegation>,
		invalid_reasons: Option<RegistrationDataError>,
	) -> Self {
		Self {
			sidechain_pub_key: to_hex(&registration_data.sidechain_pub_key.0, false),
			sidechain_account_id: Self::sidechain_account_ss58(
				registration_data.sidechain_pub_key.clone(),
			)
			.unwrap_or("Invalid Sidechain Public Key. Could not decode...".into()),
			mainchain_pub_key: to_hex(&stake_pool_public_key.0.clone(), false),
			cross_chain_pub_key: to_hex(&registration_data.cross_chain_pub_key.0, false),
			keys: keys_to_map(&registration_data.keys),
			sidechain_signature: to_hex(&registration_data.sidechain_signature.0, false),
			mainchain_signature: to_hex(&registration_data.mainchain_signature.0, false),
			cross_chain_signature: to_hex(&registration_data.cross_chain_signature.0, false),
			utxo: registration_data.utxo_info,
			stake_delegation: stake_delegation.map(|sd| sd.0),
			is_valid: invalid_reasons.is_none(),
			invalid_reasons: invalid_reasons.map(|e| e.into()),
		}
	}

	fn sidechain_account_ss58(sidechain_pub_key: SidechainPublicKey) -> Result<String, Vec<u8>> {
		let sidechain_ecdsa_pub_key =
			ecdsa::Public::from(<[u8; 33]>::try_from(sidechain_pub_key.0)?);
		let sidechain_account: AccountId32 =
			MultiSigner::from(sidechain_ecdsa_pub_key).into_account();
		let sidechain_account_ss58 = sidechain_account.to_ss58check();
		Ok(sidechain_account_ss58)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use hex_literal::hex;
	use serde_json;
	use serde_json::Value;
	use sidechain_domain::{McTxHash, McTxIndexInBlock, UtxoId};

	mod candidate_registration_entry_serialization_tests {
		use super::*;
		use sidechain_domain::{McBlockNumber, McEpochNumber, McSlotNumber, UtxoIndex, UtxoInfo};

		#[test]
		fn candidate_registration_entry_to_json() {
			let response = CandidateRegistrationEntry {
				sidechain_pub_key: "0x0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb".to_string(),
				sidechain_account_id: "5EP2cMaCxLzhfD3aFAqqgu3kfXH7GcwweEv6JXZRP6ysRHkQ".to_string(),
				mainchain_pub_key: "0x7521303029fc73ea2dd6a410c4c3cf570bf294a7e02942e049d50ba117acec22".to_string(),
				cross_chain_pub_key: "0x0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb".to_string(),
				keys: vec![("aura".to_string(), ByteString::from_hex_unsafe("0x90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22")), ("gran".to_string(),ByteString::from_hex_unsafe("439660b36c6c03afafca027b910b4fecf99801834c62a5e6006f27d978de234f"))].into_iter().collect(),
				sidechain_signature: "0x3da1014f1ba4ece29a82b98e2ee4e707bd062523f558e84857cd97d95c525ebd4762812bc1baaf92117861d41acd8641d474f1b30367f0c1ebcf0d280ec44338".to_string(),
				mainchain_signature: "0x37a45144a24ddd0ded388b7b39441b4ceb7abd1935d02fe6abf07f14025b663e81b53678b3f6701a7c76af7981246537eeee6a790aac18445bb8494bea38990f".to_string(),
				cross_chain_signature: "0x3da1014f1ba4ece29a82b98e2ee4e707bd062523f558e84857cd97d95c525ebd4762812bc1baaf92117861d41acd8641d474f1b30367f0c1ebcf0d280ec44338".to_string(),
				utxo: UtxoInfo {
					utxo_id: UtxoId {
						tx_hash: McTxHash(hex!("a40c500e3cd4a374916947bc1ff419d5ed1b3e0bef410ba793c3507703f3d6de")),
						index: UtxoIndex(0),
					},
					epoch_number: McEpochNumber(303),
					block_number: McBlockNumber(1147672),
					slot_number: McSlotNumber(26223403),
					tx_index_within_block: McTxIndexInBlock(0),
				},
				stake_delegation: Some(2380000000),
				is_valid: true,
				invalid_reasons: None,
			};
			let json_value = serde_json::to_value(response).unwrap();

			assert_eq!(
				json_value,
				serde_json::json!({
					"sidechainPubKey": "0x0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb",
					"sidechainAccountId": "5EP2cMaCxLzhfD3aFAqqgu3kfXH7GcwweEv6JXZRP6ysRHkQ",
					"mainchainPubKey": "0x7521303029fc73ea2dd6a410c4c3cf570bf294a7e02942e049d50ba117acec22",
					"crossChainPubKey": "0x0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb",
					"keys": {"aura":"0x90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22", "gran": "0x439660b36c6c03afafca027b910b4fecf99801834c62a5e6006f27d978de234f"},
					"sidechainSignature": "0x3da1014f1ba4ece29a82b98e2ee4e707bd062523f558e84857cd97d95c525ebd4762812bc1baaf92117861d41acd8641d474f1b30367f0c1ebcf0d280ec44338",
					"mainchainSignature": "0x37a45144a24ddd0ded388b7b39441b4ceb7abd1935d02fe6abf07f14025b663e81b53678b3f6701a7c76af7981246537eeee6a790aac18445bb8494bea38990f",
					"crossChainSignature": "0x3da1014f1ba4ece29a82b98e2ee4e707bd062523f558e84857cd97d95c525ebd4762812bc1baaf92117861d41acd8641d474f1b30367f0c1ebcf0d280ec44338",
					"utxo": {
						"utxoId": "a40c500e3cd4a374916947bc1ff419d5ed1b3e0bef410ba793c3507703f3d6de#0",
						"epochNumber": 303,
						"blockNumber": 1147672,
						"slotNumber": 26223403,
						"txIndexWithinBlock": 0,
					},
					"stakeDelegation": 2380000000u32,
					"isValid": true
				})
			);
		}

		#[test]
		fn should_not_display_null_values() {
			let entry = CandidateRegistrationEntry::default();
			assert!(entry.invalid_reasons.is_none());

			let json_value = serde_json::to_value(&entry).expect("Serialization failed");

			let Value::Object(map) = json_value else { panic!("should never happen") };

			// Sanity check
			assert!(map.contains_key("sidechainPubKey"));

			assert!(!map.contains_key("invalidReasons"));
		}
	}
}
