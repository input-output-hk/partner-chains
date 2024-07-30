use clap::Parser;
use pallet_session_validator_management_rpc::SessionValidatorManagementRpcApiServer;
use sc_cli::{CliConfiguration, SharedParams};
use sidechain_domain::McEpochNumber;

#[derive(Debug, Clone, Parser)]
pub struct AriadneParametersCmd {
	#[arg(long)]
	pub mc_epoch_number: McEpochNumber,
	#[allow(missing_docs)]
	#[clap(flatten)]
	pub shared_params: SharedParams,
}

impl AriadneParametersCmd {
	pub async fn run(
		&self,
		rpc: impl SessionValidatorManagementRpcApiServer,
	) -> Result<String, sc_cli::Error> {
		let ariadne_parameters = rpc.get_ariadne_parameters(self.mc_epoch_number).await;
		let json = match ariadne_parameters {
			Ok(ariadne_parameters) => serde_json::to_value(ariadne_parameters)
				.map_err(|err| sc_cli::Error::Application(err.into()))?,
			Err(err) => serde_json::json!({ "error": err.to_string() }),
		};
		let as_string = serde_json::to_string_pretty(&json)
			.map_err(|err| sc_cli::Error::Application(err.into()))?;
		println!("{as_string}");
		Ok(as_string)
	}

	pub async fn run_ignore_output(
		&self,
		rpc: impl SessionValidatorManagementRpcApiServer,
	) -> Result<(), sc_cli::Error> {
		let _ = self.run(rpc).await?;
		Ok(())
	}
}

impl CliConfiguration for AriadneParametersCmd {
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
	use jsonrpsee::types::ErrorObject;
	use pallet_session_validator_management_rpc::types::{
		AriadneParameters, CandidateRegistrationEntry, DParameter, GetCommitteeResponse,
		GetRegistrationsResponseMap, PermissionedCandidateData,
	};
	use sc_cli::TracingReceiver;
	use serde_json::Value;
	use sidechain_domain::{
		AuraPublicKey, GrandpaPublicKey, MainchainPublicKey, SidechainPublicKey,
	};

	struct MockSessionValidatorManagementRpcApiServer {
		pub ariadne_parameters: AriadneParameters,
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
			unimplemented!()
		}

		async fn get_ariadne_parameters(
			&self,
			epoch_number: McEpochNumber,
		) -> RpcResult<AriadneParameters> {
			if epoch_number.0 == 303 {
				Ok(self.ariadne_parameters.clone())
			} else {
				Err(ErrorObject::owned::<u8>(-1, "unexpected epoch number", None))
			}
		}
	}

	#[tokio::test]
	async fn should_return_correct_json_string() {
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
		let response =
			AriadneParameters { d_parameter, permissioned_candidates, candidate_registrations };

		let rpc = MockSessionValidatorManagementRpcApiServer { ariadne_parameters: response };

		let cmd_output = AriadneParametersCmd {
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
}
