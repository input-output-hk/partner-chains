//! RPC interface for session validator management
#![deny(missing_docs)]
use derive_new::new;
use jsonrpsee::{
	core::RpcResult,
	core::async_trait,
	proc_macros::rpc,
	types::{ErrorObject, ErrorObjectOwned},
};
use sidechain_domain::{McEpochNumber, StakePoolPublicKey};
use sp_session_validator_management_query::SessionValidatorManagementQueryApi;
use sp_session_validator_management_query::types::*;
use std::sync::Arc;

#[rpc(client, server, namespace = "sidechain")]
pub trait SessionValidatorManagementRpcApi {
	/// Returns the committee for given sidechain epoch. The order of the list represents the order of slot allocation.
	#[method(name = "getEpochCommittee")]
	fn get_epoch_committee(&self, epoch_number: u64) -> RpcResult<GetCommitteeResponse>;

	///
	/// returns: Last active and valid registration followed by all newer invalid registrations for mc_epoch_number and mc_public_key.
	/// Regardless of `mc_epoch_number` value, it always uses validation api from the latest sidechain block.
	///
	#[method(name = "getRegistrations")]
	async fn get_registrations(
		&self,
		mc_epoch_number: McEpochNumber,
		#[argument(rename = "mc_public_key")] stake_pool_public_key: StakePoolPublicKey,
	) -> RpcResult<Vec<CandidateRegistrationEntry>>;

	/// Regardless of `epoch_number` value, all the candidates data validation is done based on the validation api from the latest sidechain block.
	#[method(name = "getAriadneParameters")]
	async fn get_ariadne_parameters(
		&self,
		epoch_number: McEpochNumber,
	) -> RpcResult<AriadneParameters>;
}

#[derive(new)]
/// Type wrapping RPC API
pub struct SessionValidatorManagementRpc<T> {
	query_api: Arc<T>,
}

#[async_trait]
impl<T> SessionValidatorManagementRpcApiServer for SessionValidatorManagementRpc<T>
where
	T: SessionValidatorManagementQueryApi + Send + Sync + 'static,
{
	fn get_epoch_committee(&self, epoch_number: u64) -> RpcResult<GetCommitteeResponse> {
		self.query_api.get_epoch_committee(epoch_number).map_err(error_object_from_str)
	}

	async fn get_registrations(
		&self,
		mc_epoch_number: McEpochNumber,
		stake_pool_public_key: StakePoolPublicKey,
	) -> RpcResult<Vec<CandidateRegistrationEntry>> {
		self.query_api
			.get_registrations(mc_epoch_number, stake_pool_public_key)
			.await
			.map_err(error_object_from_str)
	}

	async fn get_ariadne_parameters(
		&self,
		epoch_number: McEpochNumber,
	) -> RpcResult<AriadneParameters> {
		self.query_api
			.get_ariadne_parameters(epoch_number)
			.await
			.map_err(error_object_from_str)
	}
}

fn error_object_from_str(msg: impl Into<String>) -> ErrorObjectOwned {
	ErrorObject::owned::<u8>(-1, msg, None)
}
