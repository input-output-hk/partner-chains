use crate::InherentDigest;
use futures::FutureExt;
use sp_consensus::{Environment, Proposer};
use sp_inherents::InherentData;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::{Digest, DigestItem};
use std::future::Future;
use std::marker::PhantomData;
use std::time;

/// Proposer factory for PartnerChainsProposer. Allows passing ID: InherentDigest type parameter.
pub struct PartnerChainsProposerFactory<B: BlockT, E: Environment<B>, ID> {
	env: E,
	phantom_data: PhantomData<(B, ID)>,
}

impl<B: BlockT, E: Environment<B>, ID> PartnerChainsProposerFactory<B, E, ID> {
	pub fn new(env: E) -> Self {
		Self { env, phantom_data: PhantomData }
	}
}

impl<B: BlockT, E: Environment<B>, ID: InherentDigest + Send + Sync + 'static> Environment<B>
	for PartnerChainsProposerFactory<B, E, ID>
{
	type Proposer = PartnerChainsProposer<B, E::Proposer, ID>;
	type CreateProposer =
		Box<dyn Future<Output = Result<Self::Proposer, Self::Error>> + Send + Unpin + 'static>;
	type Error = <E as Environment<B>>::Error;

	fn init(&mut self, parent_header: &<B as BlockT>::Header) -> Self::CreateProposer {
		Box::new(self.env.init(parent_header).map(|res| {
			res.map(|proposer| PartnerChainsProposer::<B, E::Proposer, ID>::new(proposer))
		}))
	}
}

/// Wraps a Proposer. Adds inherent data digest to the original logs and calls wrapped Proposer.
pub struct PartnerChainsProposer<B: BlockT, P: Proposer<B>, ID: InherentDigest> {
	pub proposer: P,
	phantom_data: PhantomData<(B, ID)>,
}

impl<B: BlockT, P: Proposer<B>, ID: InherentDigest> PartnerChainsProposer<B, P, ID> {
	pub(crate) fn new(proposer: P) -> Self {
		Self { proposer, phantom_data: PhantomData }
	}
}

impl<B: BlockT, P: Proposer<B>, ID: InherentDigest> Proposer<B>
	for PartnerChainsProposer<B, P, ID>
{
	type Error = <P as Proposer<B>>::Error;
	type Proposal = <P as Proposer<B>>::Proposal;
	type ProofRecording = <P as Proposer<B>>::ProofRecording;
	type Proof = <P as Proposer<B>>::Proof;

	fn propose(
		self,
		inherent_data: InherentData,
		inherent_digests: Digest,
		max_duration: time::Duration,
		block_size_limit: Option<usize>,
	) -> Self::Proposal {
		let mut logs: Vec<DigestItem> = Vec::from(inherent_digests.logs());
		// It is a programmatic error to try to propose a block that has inherent data from which declared InherentDigest cannot be created.
		let mut inherent_logs = ID::from_inherent_data(&inherent_data)
			.expect("InherentDigest can be created from inherent data");
		logs.append(&mut inherent_logs);
		self.proposer
			.propose(inherent_data, Digest { logs }, max_duration, block_size_limit)
	}
}

#[cfg(test)]
mod tests {
	use crate::InherentDigest;
	use crate::block_proposal::PartnerChainsProposer;
	use futures::future;
	use sp_consensus::{DisableProofRecording, Proposal, Proposer};
	use sp_inherents::InherentData;
	use sp_runtime::generic::Header;
	use sp_runtime::traits::BlakeTwo256;
	use sp_runtime::{Digest, DigestItem, OpaqueExtrinsic};
	use std::error::Error;

	pub type Block = sp_runtime::generic::Block<Header<u32, BlakeTwo256>, OpaqueExtrinsic>;

	fn expected_item() -> DigestItem {
		DigestItem::Other(vec![1, 3, 3, 7])
	}

	fn other_item() -> DigestItem {
		DigestItem::Other(vec![0, 0, 0, 0])
	}

	struct TestInherentDigest;

	impl InherentDigest for TestInherentDigest {
		type Value = ();

		fn from_inherent_data(
			_inherent_data: &InherentData,
		) -> Result<Vec<DigestItem>, Box<dyn Error + Send + Sync>> {
			Ok(vec![expected_item()])
		}

		fn value_from_digest(
			_digests: &[DigestItem],
		) -> Result<Self::Value, Box<dyn Error + Send + Sync>> {
			todo!()
		}
	}

	struct TestProposer {
		expected_digest: Digest,
	}

	impl Proposer<Block> for TestProposer {
		type Error = sp_blockchain::Error;
		type Proposal = future::Ready<Result<Proposal<Block, ()>, sp_blockchain::Error>>;
		type ProofRecording = DisableProofRecording;
		type Proof = ();

		fn propose(
			self,
			_inherent_data: InherentData,
			inherent_digests: Digest,
			_max_duration: std::time::Duration,
			_block_size_limit: Option<usize>,
		) -> Self::Proposal {
			let result = if inherent_digests != self.expected_digest {
				Err(sp_blockchain::Error::Application(
					"Inherent digest does not match expected digest".into(),
				))
			} else {
				let block = Block {
					header: Header {
						parent_hash: Default::default(),
						number: 0,
						state_root: Default::default(),
						extrinsics_root: Default::default(),
						digest: Default::default(),
					},
					extrinsics: Default::default(),
				};
				Ok(Proposal { block, proof: (), storage_changes: Default::default() })
			};
			futures::future::ready(result)
		}
	}

	#[test]
	fn inherent_digest_is_appended_to_logs() {
		let inherent_data = InherentData::new();
		let inherent_digests = Digest { logs: vec![other_item()] };
		let test_proposer =
			TestProposer { expected_digest: Digest { logs: vec![other_item(), expected_item()] } };
		let proposer: PartnerChainsProposer<Block, TestProposer, TestInherentDigest> =
			PartnerChainsProposer::new(test_proposer);
		let proposal = proposer
			.propose(inherent_data, inherent_digests, std::time::Duration::from_secs(0), None)
			.into_inner();
		assert!(proposal.is_ok());
	}
}
