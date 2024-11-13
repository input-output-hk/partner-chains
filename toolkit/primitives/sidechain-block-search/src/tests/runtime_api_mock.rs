use super::*;
use rpc_mock::*;
use sidechain_domain::*;
use sp_sidechain::SidechainStatus;

sp_api::mock_impl_runtime_apis! {
	impl GetSidechainStatus<Block> for TestRuntimeApi {
		#[advanced]
		fn get_sidechain_status(at: <Block as BlockT>::Hash) -> Result<SidechainStatus, sp_api::ApiError> {
			let block_number = conversion::block_hash_to_block_number(at.into());

			Ok(SidechainStatus {
				epoch: ScEpochNumber(conversion::get_epoch(block_number) as u64),
				slot: ScSlotNumber(conversion::get_slot(block_number) as u64),
				slots_per_epoch: conversion::SLOTS_PER_EPOCH,
			})
		}
	}
}
