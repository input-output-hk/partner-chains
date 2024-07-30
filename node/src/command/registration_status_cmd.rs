use clap::Parser;
use futures::TryFutureExt;
use pallet_session_validator_management_rpc::SessionValidatorManagementRpcApiServer;
use sc_cli::{CliConfiguration, SharedParams};
use sidechain_domain::{MainchainPublicKey, McEpochNumber};

#[derive(Debug, Clone, Parser)]
pub struct RegistrationStatusCmd {
	#[arg(long)]
	pub mainchain_pub_key: MainchainPublicKey,
	#[arg(long)]
	pub mc_epoch_number: McEpochNumber,
	#[allow(missing_docs)]
	#[clap(flatten)]
	pub shared_params: SharedParams,
}

impl RegistrationStatusCmd {
	pub async fn run(
		&self,
		rpc: impl SessionValidatorManagementRpcApiServer,
	) -> Result<String, sc_cli::Error> {
		let registrations = rpc
			.get_registrations(self.mc_epoch_number, self.mainchain_pub_key.clone())
			.map_err(|err| sc_cli::Error::Application(err.into()))
			.await?;
		let registrations_json = serde_json::to_value(registrations)
			.map_err(|err| sc_cli::Error::Application(err.into()))?;
		let pretty_registrations_json = serde_json::to_string_pretty(&registrations_json)
			.map_err(|err| sc_cli::Error::Application(err.into()))?;
		println!("{}", pretty_registrations_json);
		Ok(pretty_registrations_json)
	}

	pub async fn run_ignore_output(
		&self,
		rpc: impl SessionValidatorManagementRpcApiServer,
	) -> Result<(), sc_cli::Error> {
		let _ = self.run(rpc).await?;
		Ok(())
	}
}
impl CliConfiguration for RegistrationStatusCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use async_trait::async_trait;
	use hex_literal::hex;
	use jsonrpsee::core::RpcResult;
	use pallet_session_validator_management_rpc::types::{
		AriadneParameters, CandidateRegistrationEntry, GetCommitteeResponse,
	};
	use sc_cli::TracingReceiver;
	use serde_json::Value;
	use sidechain_domain::{
		McBlockNumber, McSlotNumber, McTxHash, McTxIndexInBlock, UtxoId, UtxoIndex, UtxoInfo,
	};
	struct MockSessionValidatorManagementRpcApiServer {
		pub expected_registrations: Vec<CandidateRegistrationEntry>,
	}

	#[async_trait]
	impl SessionValidatorManagementRpcApiServer for MockSessionValidatorManagementRpcApiServer {
		fn get_epoch_committee(&self, _: u64) -> RpcResult<GetCommitteeResponse> {
			unimplemented!()
		}

		async fn get_registrations(
			&self,
			_: McEpochNumber,
			_: MainchainPublicKey,
		) -> RpcResult<Vec<CandidateRegistrationEntry>> {
			Ok(self.expected_registrations.clone())
		}

		async fn get_ariadne_parameters(&self, _: McEpochNumber) -> RpcResult<AriadneParameters> {
			unimplemented!()
		}
	}

	#[tokio::test]
	async fn should_return_correct_json_string() {
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

		let rpc = MockSessionValidatorManagementRpcApiServer { expected_registrations: response };

		let cmd_output = RegistrationStatusCmd {
			mainchain_pub_key: MainchainPublicKey(hex!(
				"7521303029fc73ea2dd6a410c4c3cf570bf294a7e02942e049d50ba117acec22"
			)),
			mc_epoch_number: McEpochNumber(303),
			shared_params: SharedParams {
				chain: None,
				dev: false,
				base_path: None,
				log: vec![],
				detailed_log_output: false,
				disable_log_color: false,
				enable_log_reloading: false,
				tracing_targets: None,
				tracing_receiver: TracingReceiver::Log,
			},
		}
		.run(rpc)
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
