//! Types used by committee selection queries
mod ariadne;
mod registrations;

pub use ariadne::*;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
pub use registrations::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sidechain_domain::{CandidateKeys, byte_string::ByteString};
use sp_core::bytes::to_hex;
use sp_session_validator_management::CommitteeMember as CommitteeMemberT;
use std::collections::HashMap;

#[derive(
	Clone,
	Eq,
	PartialEq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Default,
	TypeInfo,
	Debug,
	Serialize,
	Deserialize,
)]
#[serde(rename_all = "camelCase")]
/// Response type for "get committee" query
pub struct GetCommitteeResponse {
	/// The sidechain epoch
	pub sidechain_epoch: u64,
	/// List of committee members
	pub committee: Vec<CommitteeMember>,
}

impl GetCommitteeResponse {
	/// Constructor for [GetCommitteeResponse]
	pub fn new<Member: CommitteeMemberT>(
		sidechain_epoch: sidechain_domain::ScEpochNumber,
		committee: Vec<Member>,
	) -> GetCommitteeResponse
	where
		Member::AuthorityId: AsRef<[u8]>,
	{
		let committee = committee
			.into_iter()
			.map(|member| CommitteeMember::new(member.authority_id()))
			.collect();
		GetCommitteeResponse { sidechain_epoch: sidechain_epoch.0, committee }
	}

	/// Legacy constructor, used for input coming from old versions of the pallet runtime
	pub fn new_legacy<AuthorityId>(
		sidechain_epoch: sidechain_domain::ScEpochNumber,
		committee: Vec<AuthorityId>,
	) -> GetCommitteeResponse
	where
		AuthorityId: AsRef<[u8]>,
	{
		let committee = committee.into_iter().map(|member| CommitteeMember::new(member)).collect();
		GetCommitteeResponse { sidechain_epoch: sidechain_epoch.0, committee }
	}
}

#[derive(
	Clone,
	Eq,
	PartialEq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Default,
	TypeInfo,
	Debug,
	Serialize,
	Deserialize,
)]
#[serde(rename_all = "camelCase")]
/// Committee member represented by their sidechain pub key
pub struct CommitteeMember {
	sidechain_pub_key: String,
}

impl CommitteeMember {
	/// Constructor for [CommitteeMember]
	pub fn new<T: AsRef<[u8]>>(bytes: T) -> Self {
		Self { sidechain_pub_key: to_hex(bytes.as_ref(), false) }
	}
}

fn keys_to_map(keys: &CandidateKeys) -> HashMap<String, ByteString> {
	keys.0
		.iter()
		.map(|key| {
			(
				String::from_utf8(key.id.to_vec()).unwrap_or("<invalid>".to_string()),
				ByteString(key.bytes.clone()),
			)
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use crate::types::GetCommitteeResponse;
	use serde_json;
	use sidechain_domain::ScEpochNumber;
	use sp_core::ecdsa;

	#[test]
	fn get_committee_response_to_json_test() {
		let input = GetCommitteeResponse::new(
			ScEpochNumber(4703884),
			vec![(ecdsa::Public::from([0u8; 33]), 0), (ecdsa::Public::from([255u8; 33]), 1)],
		);

		let json_value = serde_json::to_value(input).expect("Serialization failed");

		assert_eq!(
			json_value,
			serde_json::json!({
				"sidechainEpoch": 4703884,
				"committee": [
					{"sidechainPubKey":"0x000000000000000000000000000000000000000000000000000000000000000000"},
					{"sidechainPubKey":"0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"},
				]
			})
		);
	}
}
