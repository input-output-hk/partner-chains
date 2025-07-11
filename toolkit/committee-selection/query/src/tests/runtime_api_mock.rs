use super::*;
use crate::tests::query_mock::TestRuntimeApi;
use authority_selection_inherents::{
	PermissionedCandidateDataError, RegistrationDataError, StakeError,
	validate_permissioned_candidate_data, validate_registration_data,
};
use mock::*;
use sidechain_domain::*;
use sp_core::{Decode, Encode};
use sp_session_validator_management::MainChainScripts;
#[allow(deprecated)]
use sp_sidechain::GetSidechainStatus;
use std::str::FromStr;

pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

type SessionKeys = u64;
#[derive(Encode, Decode, Clone)]
pub struct CrossChainPublic([u8; 33]);
impl AsRef<[u8]> for CrossChainPublic {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}
impl TryFrom<SidechainPublicKey> for CrossChainPublic {
	type Error = Vec<u8>;
	fn try_from(value: SidechainPublicKey) -> Result<Self, Self::Error> {
		Ok(Self(value.0.try_into()?))
	}
}

sp_api::mock_impl_runtime_apis! {
	#[allow(deprecated)]
	impl GetSidechainStatus<Block> for TestRuntimeApi {
		#[advanced]
		fn get_sidechain_status(at: <Block as BlockT>::Hash) -> Result<SidechainStatus, sp_api::ApiError> {
			self.check_using_same_instance_for_same_block(at.encode())?;

			let block_number = conversion::block_hash_to_block_number(at.into());
			Ok(SidechainStatus {
				epoch: ScEpochNumber(conversion::get_epoch(block_number) as u64),
				slot: ScSlotNumber(conversion::get_slot(block_number) as u64),
				slots_per_epoch: conversion::SLOTS_PER_EPOCH,
			})
		}
	}
	impl SessionValidatorManagementApi<Block, (CrossChainPublic, SessionKeys), AuthoritySelectionInputs, ScEpochNumber> for TestRuntimeApi {
		#[advanced]
		fn get_current_committee(at: <Block as BlockT>::Hash) -> Result<(ScEpochNumber, sp_std::vec::Vec<(CrossChainPublic, SessionKeys)>), sp_api::ApiError> {
			self.check_using_same_instance_for_same_block(at.encode())?;
			let block_number = conversion::block_hash_to_block_number(at.into());
			let block_epoch = conversion::get_epoch(block_number);
			Ok((ScEpochNumber(block_epoch as u64), committee_for_epoch(block_epoch as u64)))
		}
		#[advanced]
		fn get_next_committee(at: <Block as BlockT>::Hash) -> Result<Option<(ScEpochNumber, sp_std::vec::Vec<(CrossChainPublic, SessionKeys)>)>, sp_api::ApiError> {
			self.check_using_same_instance_for_same_block(at.encode())?;
			let block_number = conversion::block_hash_to_block_number(at.into()) ;
			let block_epoch = conversion::get_epoch(block_number) + 1;
			Ok(Some((ScEpochNumber(block_epoch as u64), committee_for_epoch(block_epoch as u64))))
		}
		#[advanced]
		fn get_main_chain_scripts(at: <Block as BlockT>::Hash) -> Result<MainChainScripts, sp_api::ApiError> {
			self.check_using_same_instance_for_same_block(at.encode())?;
			Ok(MainChainScripts{
				committee_candidate_address: MainchainAddress::from_str("addr_0000").unwrap(),
				d_parameter_policy_id: PolicyId([1u8; 28]),
				permissioned_candidates_policy_id: PolicyId([2u8; 28]),
			})
		}
	}

	impl GetGenesisUtxo<Block> for TestRuntimeApi {
		#[advanced]
		fn genesis_utxo(at: <Block as BlockT>::Hash) -> Result<UtxoId, sp_api::ApiError> {
			self.check_using_same_instance_for_same_block(at.encode())?;
			Ok(TEST_UTXO_ID)
		}
	}

	impl CandidateValidationApi<Block> for TestRuntimeApi {
		fn validate_registered_candidate_data(mainchain_pub_key: &StakePoolPublicKey, registration_data: &RegistrationData) -> Option<RegistrationDataError> {
			validate_registration_data(mainchain_pub_key, registration_data, TEST_UTXO_ID).err()
		}
		fn validate_stake(stake: Option<StakeDelegation>) -> Option<StakeError> {
			authority_selection_inherents::validate_stake(stake).err()
		}
		fn validate_permissioned_candidate_data(candidate: sidechain_domain::PermissionedCandidateData) -> Option<PermissionedCandidateDataError> {
			validate_permissioned_candidate_data(candidate).err()
		}
	}
}

pub(crate) fn committee_for_epoch(epoch: u64) -> Vec<(CrossChainPublic, SessionKeys)> {
	if epoch == conversion::GENESIS_EPOCH || epoch == conversion::EPOCH_OF_BLOCK_1 {
		vec![(CrossChainPublic([0u8; 33]), 0), (CrossChainPublic([1u8; 33]), 1)]
	} else {
		vec![(CrossChainPublic([epoch as u8; 33]), 2), (CrossChainPublic([epoch as u8 + 1; 33]), 3)]
	}
}
