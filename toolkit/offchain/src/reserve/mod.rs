//! All smart-contracts related to Rewards Token Reserve Management

use sidechain_domain::{AssetName, PolicyId};

pub mod create;
pub mod init;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReserveToken {
	Ada,
	AssetId { policy_id: PolicyId, asset_name: AssetName },
}
