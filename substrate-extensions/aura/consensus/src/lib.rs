// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

// Additional modifications by Input Output Global, Inc.
// Copyright (C) 2024, Input Output Global, Inc.

pub mod import_queue;

use futures::prelude::*;
use parity_scale_codec::Codec;
use sc_client_api::{BlockOf, backend::AuxStore};
use sc_consensus::block_import::BlockImport;
use sc_consensus::{BlockImportParams, ForkChoiceStrategy, StateAction};
use sc_consensus_aura::{
	BuildAuraWorkerParams, CompatibilityMode, StartAuraParams, find_pre_digest,
};
use sc_consensus_slots::{
	BackoffAuthoringBlocksStrategy, InherentDataProviderExt, SimpleSlotWorkerToSlotWorker,
	SlotInfo, SlotProportion, StorageChanges,
};
use sc_telemetry::TelemetryHandle;
use sp_api::{Core, ProvideRuntimeApi};
use sp_application_crypto::AppPublic;
use sp_blockchain::HeaderBackend;
use sp_consensus::{
	BlockOrigin, Environment, Error as ConsensusError, Proposer, SelectChain, SyncOracle,
};
use sp_consensus_aura::AuraApi;
use sp_consensus_slots::Slot;
use sp_core::crypto::Pair;
use sp_inherents::CreateInherentDataProviders;
use sp_keystore::KeystorePtr;
use sp_partner_chains_consensus_aura::InherentDigest;
use sp_runtime::traits::{Block as BlockT, Header, Member, NumberFor};
use std::{fmt::Debug, marker::PhantomData, pin::Pin, sync::Arc};

type AuthorityId<P> = <P as Pair>::Public;

const LOG_TARGET: &str = "aura";

/// Start the aura worker. The returned future should be run in a futures executor.
pub fn start_aura<P, B, C, SC, I, PF, SO, L, CIDP, BS, Error, ID>(
	StartAuraParams {
		slot_duration,
		client,
		select_chain,
		block_import,
		proposer_factory,
		sync_oracle,
		justification_sync_link,
		create_inherent_data_providers,
		force_authoring,
		backoff_authoring_blocks,
		keystore,
		block_proposal_slot_portion,
		max_block_proposal_slot_portion,
		telemetry,
		compatibility_mode,
	}: StartAuraParams<C, SC, I, PF, SO, L, CIDP, BS, NumberFor<B>>,
) -> Result<impl Future<Output = ()>, ConsensusError>
where
	P: Pair,
	P::Public: AppPublic + Member,
	P::Signature: TryFrom<Vec<u8>> + Member + Codec,
	B: BlockT,
	C: ProvideRuntimeApi<B> + BlockOf + AuxStore + HeaderBackend<B> + Send + Sync,
	C::Api: AuraApi<B, AuthorityId<P>>,
	SC: SelectChain<B>,
	I: BlockImport<B> + Send + Sync + 'static,
	PF: Environment<B, Error = Error> + Send + Sync + 'static,
	PF::Proposer: Proposer<B, Error = Error>,
	SO: SyncOracle + Send + Sync + Clone,
	L: sc_consensus::JustificationSyncLink<B>,
	CIDP: CreateInherentDataProviders<B, ()> + Send + 'static,
	CIDP::InherentDataProviders: InherentDataProviderExt + Send,
	BS: BackoffAuthoringBlocksStrategy<NumberFor<B>> + Send + Sync + 'static,
	Error: std::error::Error + Send + From<ConsensusError> + 'static,
	ID: InherentDigest + Send + Sync + 'static,
{
	let worker = build_aura_worker::<P, _, _, _, _, _, _, _, _, ID>(BuildAuraWorkerParams {
		client,
		block_import,
		proposer_factory,
		keystore,
		sync_oracle: sync_oracle.clone(),
		justification_sync_link,
		force_authoring,
		backoff_authoring_blocks,
		telemetry,
		block_proposal_slot_portion,
		max_block_proposal_slot_portion,
		compatibility_mode,
	});

	Ok(sc_consensus_slots::start_slot_worker(
		slot_duration,
		select_chain,
		SimpleSlotWorkerToSlotWorker(worker),
		sync_oracle,
		create_inherent_data_providers,
	))
}

/// Build the aura worker.
///
/// The caller is responsible for running this worker, otherwise it will do nothing.
pub fn build_aura_worker<P, B, C, PF, I, SO, L, BS, Error, ID>(
	BuildAuraWorkerParams {
		client,
		block_import,
		proposer_factory,
		sync_oracle,
		justification_sync_link,
		backoff_authoring_blocks,
		keystore,
		block_proposal_slot_portion,
		max_block_proposal_slot_portion,
		telemetry,
		force_authoring,
		compatibility_mode,
	}: BuildAuraWorkerParams<C, I, PF, SO, L, BS, NumberFor<B>>,
) -> impl sc_consensus_slots::SimpleSlotWorker<
	B,
	Proposer = PF::Proposer,
	BlockImport = I,
	SyncOracle = SO,
	JustificationSyncLink = L,
	Claim = P::Public,
	AuxData = Vec<AuthorityId<P>>,
>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + BlockOf + AuxStore + HeaderBackend<B> + Send + Sync,
	C::Api: AuraApi<B, AuthorityId<P>>,
	PF: Environment<B, Error = Error> + Send + Sync + 'static,
	PF::Proposer: Proposer<B, Error = Error>,
	P: Pair,
	P::Public: AppPublic + Member,
	P::Signature: TryFrom<Vec<u8>> + Member + Codec,
	I: BlockImport<B> + Send + Sync + 'static,
	Error: std::error::Error + Send + From<ConsensusError> + 'static,
	SO: SyncOracle + Send + Sync + Clone,
	L: sc_consensus::JustificationSyncLink<B>,
	BS: BackoffAuthoringBlocksStrategy<NumberFor<B>> + Send + Sync + 'static,
	ID: InherentDigest + Send + Sync + 'static,
{
	AuraWorker {
		client,
		block_import,
		env: proposer_factory,
		keystore,
		sync_oracle,
		justification_sync_link,
		force_authoring,
		backoff_authoring_blocks,
		telemetry,
		block_proposal_slot_portion,
		max_block_proposal_slot_portion,
		compatibility_mode,
		_phantom: PhantomData::<(fn() -> P, ID)>,
	}
}

struct AuraWorker<C, E, I, P, SO, L, BS, N, ID> {
	client: Arc<C>,
	block_import: I,
	env: E,
	keystore: KeystorePtr,
	sync_oracle: SO,
	justification_sync_link: L,
	force_authoring: bool,
	backoff_authoring_blocks: Option<BS>,
	block_proposal_slot_portion: SlotProportion,
	max_block_proposal_slot_portion: Option<SlotProportion>,
	telemetry: Option<TelemetryHandle>,
	compatibility_mode: CompatibilityMode<N>,
	_phantom: PhantomData<(fn() -> P, ID)>,
}

#[async_trait::async_trait]
impl<B, C, E, I, P, Error, SO, L, BS, ID> sc_consensus_slots::SimpleSlotWorker<B>
	for AuraWorker<C, E, I, P, SO, L, BS, NumberFor<B>, ID>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + BlockOf + HeaderBackend<B> + Sync,
	C::Api: AuraApi<B, AuthorityId<P>>,
	E: Environment<B, Error = Error> + Send + Sync,
	E::Proposer: Proposer<B, Error = Error>,
	I: BlockImport<B> + Send + Sync + 'static,
	P: Pair,
	P::Public: AppPublic + Member,
	P::Signature: TryFrom<Vec<u8>> + Member + Codec,
	SO: SyncOracle + Send + Clone + Sync,
	L: sc_consensus::JustificationSyncLink<B>,
	BS: BackoffAuthoringBlocksStrategy<NumberFor<B>> + Send + Sync + 'static,
	Error: std::error::Error + Send + From<ConsensusError> + 'static,
	ID: InherentDigest + Send + Sync + 'static,
{
	type BlockImport = I;
	type SyncOracle = SO;
	type JustificationSyncLink = L;
	type CreateProposer =
		Pin<Box<dyn Future<Output = Result<E::Proposer, ConsensusError>> + Send + 'static>>;
	type Proposer = E::Proposer;
	type Claim = P::Public;
	type AuxData = Vec<AuthorityId<P>>;

	fn logging_target(&self) -> &'static str {
		"aura"
	}

	fn block_import(&mut self) -> &mut Self::BlockImport {
		&mut self.block_import
	}

	fn aux_data(&self, header: &B::Header, _slot: Slot) -> Result<Self::AuxData, ConsensusError> {
		authorities(
			self.client.as_ref(),
			header.hash(),
			*header.number() + 1u32.into(),
			&self.compatibility_mode,
		)
	}

	fn authorities_len(&self, authorities: &Self::AuxData) -> Option<usize> {
		Some(authorities.len())
	}

	async fn claim_slot(
		&mut self,
		_header: &B::Header,
		slot: Slot,
		authorities: &Self::AuxData,
	) -> Option<Self::Claim> {
		sc_consensus_aura::standalone::claim_slot::<P>(slot, authorities, &self.keystore).await
	}

	fn pre_digest_data(&self, slot: Slot, _claim: &Self::Claim) -> Vec<sp_runtime::DigestItem> {
		vec![sc_consensus_aura::standalone::pre_digest::<P>(slot)]
	}

	async fn block_import_params(
		&self,
		header: B::Header,
		header_hash: &B::Hash,
		body: Vec<B::Extrinsic>,
		storage_changes: StorageChanges<B>,
		public: Self::Claim,
		_authorities: Self::AuxData,
	) -> Result<BlockImportParams<B>, ConsensusError> {
		let signature_digest_item =
			sc_consensus_aura::standalone::seal::<_, P>(header_hash, &public, &self.keystore)?;

		let mut import_block = BlockImportParams::new(BlockOrigin::Own, header);
		import_block.post_digests.push(signature_digest_item);
		import_block.body = Some(body);
		import_block.state_action =
			StateAction::ApplyChanges(sc_consensus::StorageChanges::Changes(storage_changes));
		import_block.fork_choice = Some(ForkChoiceStrategy::LongestChain);

		Ok(import_block)
	}

	fn force_authoring(&self) -> bool {
		self.force_authoring
	}

	fn should_backoff(&self, slot: Slot, chain_head: &B::Header) -> bool {
		if let Some(ref strategy) = self.backoff_authoring_blocks {
			if let Ok(chain_head_slot) = find_pre_digest::<B, P::Signature>(chain_head) {
				return strategy.should_backoff(
					*chain_head.number(),
					chain_head_slot,
					self.client.info().finalized_number,
					slot,
					self.logging_target(),
				);
			}
		}
		false
	}

	fn sync_oracle(&mut self) -> &mut Self::SyncOracle {
		&mut self.sync_oracle
	}

	fn justification_sync_link(&mut self) -> &mut Self::JustificationSyncLink {
		&mut self.justification_sync_link
	}

	fn proposer(&mut self, block: &B::Header) -> Self::CreateProposer {
		self.env
			.init(block)
			.map_err(|e| ConsensusError::ClientImport(format!("{:?}", e)))
			.boxed()
	}

	fn telemetry(&self) -> Option<TelemetryHandle> {
		self.telemetry.clone()
	}

	fn proposing_remaining_duration(&self, slot_info: &SlotInfo<B>) -> std::time::Duration {
		let parent_slot = find_pre_digest::<B, P::Signature>(&slot_info.chain_head).ok();

		sc_consensus_slots::proposing_remaining_duration(
			parent_slot,
			slot_info,
			&self.block_proposal_slot_portion,
			self.max_block_proposal_slot_portion.as_ref(),
			sc_consensus_slots::SlotLenienceType::Exponential,
			self.logging_target(),
		)
	}
}

fn authorities<A, B, C>(
	client: &C,
	parent_hash: B::Hash,
	context_block_number: NumberFor<B>,
	compatibility_mode: &CompatibilityMode<NumberFor<B>>,
) -> Result<Vec<A>, ConsensusError>
where
	A: Codec + Debug,
	B: BlockT,
	C: ProvideRuntimeApi<B>,
	C::Api: AuraApi<B, A>,
{
	let runtime_api = client.runtime_api();

	match compatibility_mode {
		CompatibilityMode::None => {},
		// Use `initialize_block` until we hit the block that should disable the mode.
		CompatibilityMode::UseInitializeBlock { until } => {
			if *until > context_block_number {
				runtime_api
					.initialize_block(
						parent_hash,
						&B::Header::new(
							context_block_number,
							Default::default(),
							Default::default(),
							parent_hash,
							Default::default(),
						),
					)
					.map_err(|_| ConsensusError::InvalidAuthoritiesSet)?;
			}
		},
	}

	runtime_api
		.authorities(parent_hash)
		.ok()
		.ok_or(ConsensusError::InvalidAuthoritiesSet)
}

#[cfg(test)]
mod tests {
	use super::*;
	use parking_lot::Mutex;
	use sc_block_builder::BlockBuilderBuilder;
	use sc_client_api::BlockchainEvents;
	use sc_consensus::BoxJustificationImport;
	use sc_consensus_aura::{CheckForEquivocation, standalone::slot_duration};
	use sc_consensus_slots::{BackoffAuthoringOnFinalizedHeadLagging, SimpleSlotWorker};
	use sc_keystore::LocalKeystore;
	use sc_network_test::{Block as TestBlock, *};
	use sp_application_crypto::{AppCrypto, key_types::AURA};
	use sp_consensus::{DisableProofRecording, NoNetwork as DummyOracle, Proposal};
	use sp_consensus_aura::SlotDuration;
	use sp_consensus_aura::inherents::InherentDataProvider;
	use sp_consensus_aura::sr25519::AuthorityPair;
	use sp_inherents::InherentData;
	use sp_keyring::sr25519::Keyring;
	use sp_keystore::Keystore;
	use sp_partner_chains_consensus_aura::CurrentSlotProvider;
	use sp_runtime::{
		Digest,
		traits::{Block as BlockT, Header as _},
	};
	use sp_timestamp::Timestamp;
	use std::{
		task::Poll,
		time::{Duration, Instant},
	};
	use substrate_test_runtime_client::{
		TestClient,
		runtime::{H256, Header},
	};

	const SLOT_DURATION_MS: u64 = 1000;

	type Error = sp_blockchain::Error;

	struct DummyFactory(Arc<TestClient>);
	struct DummyProposer(Arc<TestClient>);

	impl Environment<TestBlock> for DummyFactory {
		type Proposer = DummyProposer;
		type CreateProposer = futures::future::Ready<Result<DummyProposer, Error>>;
		type Error = Error;

		fn init(&mut self, _: &<TestBlock as BlockT>::Header) -> Self::CreateProposer {
			futures::future::ready(Ok(DummyProposer(self.0.clone())))
		}
	}

	impl Proposer<TestBlock> for DummyProposer {
		type Error = Error;
		type Proposal = future::Ready<Result<Proposal<TestBlock, ()>, Error>>;
		type ProofRecording = DisableProofRecording;
		type Proof = ();

		fn propose(
			self,
			_: InherentData,
			digests: Digest,
			_: Duration,
			_: Option<usize>,
		) -> Self::Proposal {
			let r = BlockBuilderBuilder::new(&*self.0)
				.on_parent_block(self.0.chain_info().best_hash)
				.fetch_parent_block_number(&*self.0)
				.unwrap()
				.with_inherent_digests(digests)
				.build()
				.unwrap()
				.build();

			future::ready(r.map(|b| Proposal {
				block: b.block,
				proof: (),
				storage_changes: b.storage_changes,
			}))
		}
	}

	type AuraVerifier =
		import_queue::AuraVerifier<PeersFullClient, AuthorityPair, TestCIDP, u64, ()>;
	type AuraPeer = Peer<(), PeersClient>;

	#[derive(Default)]
	pub struct AuraTestNet {
		peers: Vec<AuraPeer>,
	}

	pub struct TestCIDP;

	#[async_trait::async_trait]
	impl CreateInherentDataProviders<Block, (Slot, ())> for TestCIDP {
		type InherentDataProviders = ();

		async fn create_inherent_data_providers(
			&self,
			_parent: <Block as BlockT>::Hash,
			_extra_args: (Slot, ()),
		) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
			Ok(())
		}
	}

	impl CurrentSlotProvider for TestCIDP {
		fn slot(&self) -> Slot {
			Slot::from_timestamp(Timestamp::current(), SlotDuration::from_millis(SLOT_DURATION_MS))
		}
	}

	impl TestNetFactory for AuraTestNet {
		type Verifier = AuraVerifier;
		type PeerData = ();
		type BlockImport = PeersClient;

		fn make_verifier(&self, client: PeersClient, _peer_data: &()) -> Self::Verifier {
			let client = client.as_client();
			let slot_duration = slot_duration(&*client).expect("slot duration available");

			assert_eq!(slot_duration.as_millis() as u64, SLOT_DURATION_MS);
			AuraVerifier::new(
				client,
				TestCIDP,
				CheckForEquivocation::Yes,
				None,
				CompatibilityMode::None,
			)
		}

		fn make_block_import(
			&self,
			client: PeersClient,
		) -> (
			BlockImportAdapter<Self::BlockImport>,
			Option<BoxJustificationImport<Block>>,
			Self::PeerData,
		) {
			(client.as_block_import(), None, ())
		}

		fn peer(&mut self, i: usize) -> &mut AuraPeer {
			&mut self.peers[i]
		}

		fn peers(&self) -> &Vec<AuraPeer> {
			&self.peers
		}

		fn peers_mut(&mut self) -> &mut Vec<AuraPeer> {
			&mut self.peers
		}

		fn mut_peers<F: FnOnce(&mut Vec<AuraPeer>)>(&mut self, closure: F) {
			closure(&mut self.peers);
		}
	}

	#[tokio::test]
	async fn authoring_blocks() {
		sp_tracing::try_init_simple();
		let net = AuraTestNet::new(3);

		let peers = &[(0, Keyring::Alice), (1, Keyring::Bob), (2, Keyring::Charlie)];

		let net = Arc::new(Mutex::new(net));
		let mut import_notifications = Vec::new();
		let mut aura_futures = Vec::new();

		let mut keystore_paths = Vec::new();
		for (peer_id, key) in peers {
			let mut net = net.lock();
			let peer = net.peer(*peer_id);
			let client = peer.client().as_client();
			let select_chain = peer.select_chain().expect("full client has a select chain");
			let keystore_path = tempfile::tempdir().expect("Creates keystore path");
			let keystore = Arc::new(
				LocalKeystore::open(keystore_path.path(), None).expect("Creates keystore."),
			);

			keystore
				.sr25519_generate_new(AURA, Some(&key.to_seed()))
				.expect("Creates authority key");
			keystore_paths.push(keystore_path);

			let environ = DummyFactory(client.clone());
			import_notifications.push(
				client
					.import_notification_stream()
					.take_while(|n| {
						future::ready(!(n.origin != BlockOrigin::Own && n.header.number() < &5))
					})
					.for_each(move |_| future::ready(())),
			);

			let slot_duration = slot_duration(&*client).expect("slot duration available");

			aura_futures.push(
				start_aura::<AuthorityPair, _, _, _, _, _, _, _, _, _, _, ()>(StartAuraParams {
					slot_duration,
					block_import: client.clone(),
					select_chain,
					client,
					proposer_factory: environ,
					sync_oracle: DummyOracle,
					justification_sync_link: (),
					create_inherent_data_providers: |_, _| async {
						let slot = InherentDataProvider::from_timestamp_and_slot_duration(
							Timestamp::current(),
							SlotDuration::from_millis(SLOT_DURATION_MS),
						);

						Ok((slot,))
					},
					force_authoring: false,
					backoff_authoring_blocks: Some(
						BackoffAuthoringOnFinalizedHeadLagging::default(),
					),
					keystore,
					block_proposal_slot_portion: SlotProportion::new(0.5),
					max_block_proposal_slot_portion: None,
					telemetry: None,
					compatibility_mode: CompatibilityMode::None,
				})
				.expect("Starts aura"),
			);
		}

		future::select(
			future::poll_fn(move |cx| {
				net.lock().poll(cx);
				Poll::<()>::Pending
			}),
			future::select(future::join_all(aura_futures), future::join_all(import_notifications)),
		)
		.await;
	}

	#[tokio::test]
	async fn current_node_authority_should_claim_slot() {
		let net = AuraTestNet::new(4);

		let mut authorities = vec![
			Keyring::Alice.public().into(),
			Keyring::Bob.public().into(),
			Keyring::Charlie.public().into(),
		];

		let keystore_path = tempfile::tempdir().expect("Creates keystore path");
		let keystore = LocalKeystore::open(keystore_path.path(), None).expect("Creates keystore.");
		let public = keystore
			.sr25519_generate_new(AuthorityPair::ID, None)
			.expect("Key should be created");
		authorities.push(public.into());

		let net = Arc::new(Mutex::new(net));

		let mut net = net.lock();
		let peer = net.peer(3);
		let client = peer.client().as_client();
		let environ = DummyFactory(client.clone());

		let mut worker = AuraWorker {
			client: client.clone(),
			block_import: client,
			env: environ,
			keystore: keystore.into(),
			sync_oracle: DummyOracle,
			justification_sync_link: (),
			force_authoring: false,
			backoff_authoring_blocks: Some(BackoffAuthoringOnFinalizedHeadLagging::default()),
			telemetry: None,
			block_proposal_slot_portion: SlotProportion::new(0.5),
			max_block_proposal_slot_portion: None,
			compatibility_mode: Default::default(),
			_phantom: PhantomData::<(fn() -> AuthorityPair, ())>,
		};

		let head = Header::new(
			1,
			H256::from_low_u64_be(0),
			H256::from_low_u64_be(0),
			Default::default(),
			Default::default(),
		);
		assert!(worker.claim_slot(&head, 0.into(), &authorities).await.is_none());
		assert!(worker.claim_slot(&head, 1.into(), &authorities).await.is_none());
		assert!(worker.claim_slot(&head, 2.into(), &authorities).await.is_none());
		assert!(worker.claim_slot(&head, 3.into(), &authorities).await.is_some());
		assert!(worker.claim_slot(&head, 4.into(), &authorities).await.is_none());
		assert!(worker.claim_slot(&head, 5.into(), &authorities).await.is_none());
		assert!(worker.claim_slot(&head, 6.into(), &authorities).await.is_none());
		assert!(worker.claim_slot(&head, 7.into(), &authorities).await.is_some());
	}

	#[tokio::test]
	async fn on_slot_returns_correct_block() {
		let net = AuraTestNet::new(4);

		let keystore_path = tempfile::tempdir().expect("Creates keystore path");
		let keystore = LocalKeystore::open(keystore_path.path(), None).expect("Creates keystore.");
		keystore
			.sr25519_generate_new(AuthorityPair::ID, Some(&Keyring::Alice.to_seed()))
			.expect("Key should be created");

		let net = Arc::new(Mutex::new(net));

		let mut net = net.lock();
		let peer = net.peer(3);
		let client = peer.client().as_client();
		let environ = DummyFactory(client.clone());

		let mut worker = AuraWorker {
			client: client.clone(),
			block_import: client.clone(),
			env: environ,
			keystore: keystore.into(),
			sync_oracle: DummyOracle,
			justification_sync_link: (),
			force_authoring: false,
			backoff_authoring_blocks: Option::<()>::None,
			telemetry: None,
			block_proposal_slot_portion: SlotProportion::new(0.5),
			max_block_proposal_slot_portion: None,
			compatibility_mode: Default::default(),
			_phantom: PhantomData::<(fn() -> AuthorityPair, ())>,
		};

		let head = client.expect_header(client.info().genesis_hash).unwrap();

		let res = worker
			.on_slot(SlotInfo {
				slot: 0.into(),
				ends_at: Instant::now() + Duration::from_secs(100),
				create_inherent_data: Box::new(()),
				duration: Duration::from_millis(1000),
				chain_head: head,
				block_size_limit: None,
			})
			.await
			.unwrap();

		// The returned block should be imported and we should be able to get its header by now.
		assert!(client.header(res.block.hash()).unwrap().is_some());
	}
}
