mod ariadne;
mod registrations;

pub use ariadne::*;
use parity_scale_codec::{Decode, Encode};
pub use registrations::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::bytes::to_hex;

#[derive(
	Clone, Eq, PartialEq, Encode, Decode, Default, TypeInfo, Debug, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct GetCommitteeResponse {
	pub sidechain_epoch: u64,
	pub committee: Vec<CommitteeMember>,
}

impl GetCommitteeResponse {
	pub fn new<CrossChainPublic: AsRef<[u8]>>(
		sidechain_epoch: sidechain_domain::ScEpochNumber,
		committee: Vec<CrossChainPublic>,
	) -> GetCommitteeResponse {
		let committee: Vec<CommitteeMember> =
			committee.into_iter().map(CommitteeMember::new).collect();
		GetCommitteeResponse { sidechain_epoch: sidechain_epoch.0, committee }
	}
}

#[derive(
	Clone, Eq, PartialEq, Encode, Decode, Default, TypeInfo, Debug, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct CommitteeMember {
	sidechain_pub_key: String,
}

impl CommitteeMember {
	pub fn new<T: AsRef<[u8]>>(bytes: T) -> Self {
		Self { sidechain_pub_key: to_hex(bytes.as_ref(), false) }
	}
}

#[cfg(test)]
mod tests {
	use crate::GetCommitteeResponse;
	use serde_json;
	use sidechain_domain::ScEpochNumber;
	use sp_core::ecdsa;

	#[test]
	fn get_committee_response_to_json_test() {
		let input = GetCommitteeResponse::new(
			ScEpochNumber(4703884),
			vec![ecdsa::Public::from([0u8; 33]), ecdsa::Public::from([255u8; 33])],
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
