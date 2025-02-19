use crate::SessionValidatorManagementQueryApi;
use sidechain_domain::{McEpochNumber, StakePoolPublicKey};

pub async fn cli_get_ariadne_parameters(
	query: impl SessionValidatorManagementQueryApi,
	mc_epoch_number: McEpochNumber,
) -> Result<String, String> {
	let ariadne_parameters = query
		.get_ariadne_parameters(mc_epoch_number)
		.await
		.map_err(|err| err.to_string())?;
	let json = serde_json::to_value(ariadne_parameters).map_err(|err| err.to_string())?;
	serde_json::to_string_pretty(&json).map_err(|err| err.to_string())
}

pub async fn cli_get_registration_status(
	query: impl SessionValidatorManagementQueryApi,
	mc_epoch_number: McEpochNumber,
	stake_pool_public_key: StakePoolPublicKey,
) -> Result<String, String> {
	let registrations = query
		.get_registrations(mc_epoch_number, stake_pool_public_key.clone())
		.await
		.map_err(|err| err.to_string())?;
	let registrations_json = serde_json::to_value(registrations).map_err(|err| err.to_string())?;
	serde_json::to_string_pretty(&registrations_json).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		types::{
			AriadneParameters, CandidateRegistrationEntry, DParameter, GetCommitteeResponse,
			GetRegistrationsResponseMap, PermissionedCandidateData,
		},
		QueryResult,
	};
	use async_trait::async_trait;
	use hex_literal::hex;
	use serde_json::Value;
	use sidechain_domain::{
		AuraPublicKey, GrandpaPublicKey, McBlockNumber, McSlotNumber, McTxHash, McTxIndexInBlock,
		SidechainPublicKey, StakePoolPublicKey, UtxoId, UtxoIndex, UtxoInfo,
	};

	struct MockSessionValidatorManagementQuery {
		pub ariadne_parameters: Option<AriadneParameters>,
		pub expected_registrations: Vec<CandidateRegistrationEntry>,
	}

	#[async_trait]
	impl SessionValidatorManagementQueryApi for MockSessionValidatorManagementQuery {
		fn get_epoch_committee(&self, _: u64) -> QueryResult<GetCommitteeResponse> {
			unimplemented!()
		}

		async fn get_registrations(
			&self,
			_: McEpochNumber,
			_: StakePoolPublicKey,
		) -> QueryResult<Vec<CandidateRegistrationEntry>> {
			Ok(self.expected_registrations.clone())
		}

		async fn get_ariadne_parameters(
			&self,
			epoch_number: McEpochNumber,
		) -> QueryResult<AriadneParameters> {
			if epoch_number.0 == 303 {
				Ok(self.ariadne_parameters.clone().unwrap())
			} else {
				Err("unexpected epoch number".into())
			}
		}
	}

	#[tokio::test]
	async fn ariadne_parameters_returns_correct_json_string() {
		let d_parameter =
			DParameter { num_permissioned_candidates: 1, num_registered_candidates: 2 };
		let permissioned_candidates = vec![PermissionedCandidateData {
			sidechain_public_key: SidechainPublicKey(
				hex!("0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb").to_vec(),
			),
			aura_public_key: AuraPublicKey(
				hex!("90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22").to_vec(),
			),
			grandpa_public_key: GrandpaPublicKey(
				hex!("439660b36c6c03afafca027b910b4fecf99801834c62a5e6006f27d978de234f").to_vec(),
			),
			is_valid: true,
			invalid_reasons: None,
		}];
		let candidate_registrations = GetRegistrationsResponseMap::new();
		let ariadne_parameters =
			AriadneParameters { d_parameter, permissioned_candidates, candidate_registrations };

		let query = MockSessionValidatorManagementQuery {
			ariadne_parameters: Some(ariadne_parameters),
			expected_registrations: vec![],
		};

		let cmd_output = cli_get_ariadne_parameters(query, McEpochNumber(303)).await.unwrap();

		assert_eq!(
			serde_json::from_str::<Value>(&cmd_output).unwrap(),
			serde_json::json!({
				"dParameter": {
					"numPermissionedCandidates": 1,
					"numRegisteredCandidates": 2
				},
				"candidateRegistrations": {},
				"permissionedCandidates":[
					{
						"auraPublicKey": "0x90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22",
						"grandpaPublicKey": "0x439660b36c6c03afafca027b910b4fecf99801834c62a5e6006f27d978de234f",
						"sidechainPublicKey": "0x0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb",
						"isValid": true
					}
				]
			})
		);
	}

	#[tokio::test]
	async fn cli_get_registration_status_returns_correct_json_string() {
		let response = vec![CandidateRegistrationEntry {
			sidechain_pub_key: "0x0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb".to_string(),
			sidechain_account_id: "5EP2cMaCxLzhfD3aFAqqgu3kfXH7GcwweEv6JXZRP6ysRHkQ".to_string(),
			mainchain_pub_key: "0x7521303029fc73ea2dd6a410c4c3cf570bf294a7e02942e049d50ba117acec22".to_string(),
			cross_chain_pub_key: "0x0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb".to_string(),
			aura_pub_key: "90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22".to_string(),
			grandpa_pub_key: "439660b36c6c03afafca027b910b4fecf99801834c62a5e6006f27d978de234f".to_string(),
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
		}];

		let query = MockSessionValidatorManagementQuery {
			ariadne_parameters: None,
			expected_registrations: response,
		};

		let cmd_output = cli_get_registration_status(
			query,
			McEpochNumber(303),
			StakePoolPublicKey(hex!(
				"7521303029fc73ea2dd6a410c4c3cf570bf294a7e02942e049d50ba117acec22"
			)),
		)
		.await
		.unwrap();

		assert_eq!(
			serde_json::from_str::<Value>(&cmd_output).unwrap(),
			serde_json::json!([{
				"sidechainPubKey": "0x0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb",
				"sidechainAccountId": "5EP2cMaCxLzhfD3aFAqqgu3kfXH7GcwweEv6JXZRP6ysRHkQ",
				"mainchainPubKey": "0x7521303029fc73ea2dd6a410c4c3cf570bf294a7e02942e049d50ba117acec22",
				"crossChainPubKey": "0x0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb",
				"auraPubKey": "90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22",
				"grandpaPubKey": "439660b36c6c03afafca027b910b4fecf99801834c62a5e6006f27d978de234f",
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
			}])
		);
	}
}
