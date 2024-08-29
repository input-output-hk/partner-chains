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

//! Module implementing the logic for verifying and importing AuRa blocks.

use crate::{authorities, AuthorityId, LOG_TARGET};
use log::{debug, info, trace};
use parity_scale_codec::Codec;
use sc_client_api::{backend::AuxStore, BlockOf, UsageProvider};
use sc_consensus::{
	block_import::{BlockImport, BlockImportParams, ForkChoiceStrategy},
	import_queue::{BasicQueue, DefaultImportQueue, Verifier},
};
use sc_consensus_aura::{
	standalone::SealVerificationError, CheckForEquivocation, CompatibilityMode, Error,
	ImportQueueParams,
};
use sc_consensus_slots::{check_equivocation, CheckedHeader};
use sc_telemetry::{telemetry, TelemetryHandle, CONSENSUS_DEBUG, CONSENSUS_TRACE};
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::Error as ConsensusError;
use sp_consensus_aura::AuraApi;
use sp_consensus_slots::Slot;
use sp_core::crypto::Pair;
use sp_inherents::{CreateInherentDataProviders, InherentData, InherentDataProvider};
use sp_partner_chains_consensus_aura::{CurrentSlotProvider, InherentDigest};
use sp_runtime::{
	traits::{Block as BlockT, Header, NumberFor},
	DigestItem,
};
use std::{fmt::Debug, marker::PhantomData, sync::Arc};

/// check a header has been signed by the right key. If the slot is too far in the future, an error
/// will be returned. If it's successful, returns the pre-header and the digest item
/// containing the seal.
///
/// This digest item will always return `Some` when used with `as_aura_seal`.
fn check_header<C, B: BlockT, P: Pair>(
	client: &C,
	slot_now: Slot,
	header: B::Header,
	hash: B::Hash,
	authorities: &[AuthorityId<P>],
	check_for_equivocation: CheckForEquivocation,
) -> Result<CheckedHeader<B::Header, (Slot, DigestItem)>, Error<B>>
where
	P::Public: Codec,
	P::Signature: Codec,
	C: AuxStore,
{
	let check_result = sc_consensus_aura::standalone::check_header_slot_and_seal::<B, P>(
		slot_now,
		header,
		authorities,
	);

	match check_result {
		Ok((header, slot, seal)) => {
			let expected_author =
				sc_consensus_aura::standalone::slot_author::<P>(slot, &authorities);
			let should_equiv_check = matches!(check_for_equivocation, CheckForEquivocation::Yes);
			if let (true, Some(expected)) = (should_equiv_check, expected_author) {
				if let Some(equivocation_proof) =
					check_equivocation(client, slot_now, slot, &header, expected)
						.map_err(Error::Client)?
				{
					info!(
						target: LOG_TARGET,
						"Slot author is equivocating at slot {} with headers {:?} and {:?}",
						slot,
						equivocation_proof.first_header.hash(),
						equivocation_proof.second_header.hash(),
					);
				}
			}

			Ok(CheckedHeader::Checked(header, (slot, seal)))
		},
		Err(SealVerificationError::Deferred(header, slot)) => {
			Ok(CheckedHeader::Deferred(header, slot))
		},
		Err(SealVerificationError::Unsealed) => Err(Error::HeaderUnsealed(hash)),
		Err(SealVerificationError::BadSeal) => Err(Error::HeaderBadSeal(hash)),
		Err(SealVerificationError::BadSignature) => Err(Error::BadSignature(hash)),
		Err(SealVerificationError::SlotAuthorNotFound) => Err(Error::SlotAuthorNotFound),
		Err(SealVerificationError::InvalidPreDigest(e)) => Err(Error::from(e)),
	}
}

/// A verifier for Aura blocks, with added ID phantom type.
pub struct AuraVerifier<C, P, CIDP, N, ID> {
	client: Arc<C>,
	create_inherent_data_providers: CIDP,
	check_for_equivocation: CheckForEquivocation,
	telemetry: Option<TelemetryHandle>,
	compatibility_mode: CompatibilityMode<N>,
	_phantom: PhantomData<(fn() -> P, ID)>,
}

impl<C, P, CIDP, N, ID> AuraVerifier<C, P, CIDP, N, ID> {
	pub(crate) fn new(
		client: Arc<C>,
		create_inherent_data_providers: CIDP,
		check_for_equivocation: CheckForEquivocation,
		telemetry: Option<TelemetryHandle>,
		compatibility_mode: CompatibilityMode<N>,
	) -> Self {
		Self {
			client,
			create_inherent_data_providers,
			check_for_equivocation,
			telemetry,
			compatibility_mode,
			_phantom: PhantomData,
		}
	}
}

impl<C, P, CIDP, N, ID> AuraVerifier<C, P, CIDP, N, ID>
where
	CIDP: Send,
{
	async fn check_inherents<B: BlockT>(
		&self,
		block: B,
		at_hash: B::Hash,
		inherent_data_providers: CIDP::InherentDataProviders,
	) -> Result<(), Error<B>>
	where
		C: ProvideRuntimeApi<B>,
		C::Api: BlockBuilderApi<B>,
		CIDP: CreateInherentDataProviders<B, (Slot, <ID as InherentDigest>::Value)>,
		ID: InherentDigest,
	{
		let inherent_data = create_inherent_data::<B>(&inherent_data_providers).await?;

		let inherent_res = self
			.client
			.runtime_api()
			.check_inherents(at_hash, block, inherent_data)
			.map_err(|e| Error::Client(e.into()))?;

		if !inherent_res.ok() {
			for (i, e) in inherent_res.into_errors() {
				match inherent_data_providers.try_handle_error(&i, &e).await {
					Some(res) => res.map_err(Error::Inherent)?,
					None => return Err(Error::UnknownInherentError(i)),
				}
			}
		}

		Ok(())
	}
}

#[async_trait::async_trait]
impl<B: BlockT, C, P, CIDP, ID> Verifier<B> for AuraVerifier<C, P, CIDP, NumberFor<B>, ID>
where
	C: ProvideRuntimeApi<B> + Send + Sync + AuxStore,
	C::Api: BlockBuilderApi<B> + AuraApi<B, AuthorityId<P>> + ApiExt<B>,
	P: Pair,
	P::Public: Codec + Debug,
	P::Signature: Codec,
	CIDP: CurrentSlotProvider
		+ CreateInherentDataProviders<B, (Slot, <ID as InherentDigest>::Value)>
		+ Send
		+ Sync,
	ID: InherentDigest + Send + Sync + 'static,
{
	async fn verify(
		&self,
		mut block: BlockImportParams<B>,
	) -> Result<BlockImportParams<B>, String> {
		// Skip checks that include execution, if being told so or when importing only state.
		//
		// This is done for example when gap syncing and it is expected that the block after the gap
		// was checked/chosen properly, e.g. by warp syncing to this block using a finality proof.
		// Or when we are importing state only and can not verify the seal.
		if block.with_state() || block.state_action.skip_execution_checks() {
			// When we are importing only the state of a block, it will be the best block.
			block.fork_choice = Some(ForkChoiceStrategy::Custom(block.with_state()));

			return Ok(block);
		}

		let hash = block.header.hash();
		let parent_hash = *block.header.parent_hash();
		let authorities = authorities(
			self.client.as_ref(),
			parent_hash,
			*block.header.number(),
			&self.compatibility_mode,
		)
		.map_err(|e| format!("Could not fetch authorities at {:?}: {}", parent_hash, e))?;

		let slot_now = self.create_inherent_data_providers.slot();

		// we add one to allow for some small drift.
		// FIXME #1019 in the future, alter this queue to allow deferring of
		// headers
		let checked_header = check_header::<C, B, P>(
			&self.client,
			slot_now + 1,
			block.header.clone(),
			hash,
			&authorities[..],
			self.check_for_equivocation,
		)
		.map_err(|e| e.to_string())?;
		let inherent_digest = <ID as InherentDigest>::value_from_digest(
			block.header.digest().logs(),
		)
		.map_err(|e| {
			format!("Failed to retrieve inherent digest from header at {:?}: {}", parent_hash, e)
		})?;
		match checked_header {
			CheckedHeader::Checked(pre_header, (slot, seal)) => {
				// if the body is passed through, we need to use the runtime
				// to check that the internally-set timestamp in the inherents
				// actually matches the slot set in the seal.
				if let Some(inner_body) = block.body.take() {
					let new_block = B::new(pre_header.clone(), inner_body);

					let inherent_data_providers = create_inherent_data_provider(
						&self.create_inherent_data_providers,
						parent_hash,
						(slot, inherent_digest),
					)
					.await?;

					// skip the inherents verification if the runtime API is old or not expected to
					// exist.
					if self
						.client
						.runtime_api()
						.has_api_with::<dyn BlockBuilderApi<B>, _>(parent_hash, |v| v >= 2)
						.map_err(|e| e.to_string())?
					{
						self.check_inherents(
							new_block.clone(),
							parent_hash,
							inherent_data_providers,
						)
						.await
						.map_err(|e| e.to_string())?;
					}

					let (_, inner_body) = new_block.deconstruct();
					block.body = Some(inner_body);
				}

				trace!(target: LOG_TARGET, "Checked {:?}; importing.", pre_header);
				telemetry!(
					self.telemetry;
					CONSENSUS_TRACE;
					"aura.checked_and_importing";
					"pre_header" => ?pre_header,
				);

				block.header = pre_header;
				block.post_digests.push(seal);
				block.fork_choice = Some(ForkChoiceStrategy::LongestChain);
				block.post_hash = Some(hash);

				Ok(block)
			},
			CheckedHeader::Deferred(a, b) => {
				debug!(target: LOG_TARGET, "Checking {:?} failed; {:?}, {:?}.", hash, a, b);
				telemetry!(
					self.telemetry;
					CONSENSUS_DEBUG;
					"aura.header_too_far_in_future";
					"hash" => ?hash,
					"a" => ?a,
					"b" => ?b,
				);
				Err(format!("Header {:?} rejected: too far in the future", hash))
			},
		}
	}
}

/// Start an import queue for the Aura consensus algorithm.
pub fn import_queue<P, Block, I, C, S, CIDP, ID>(
	ImportQueueParams {
		block_import,
		justification_import,
		client,
		create_inherent_data_providers,
		spawner,
		registry,
		check_for_equivocation,
		telemetry,
		compatibility_mode,
	}: ImportQueueParams<Block, I, C, S, CIDP>,
) -> Result<DefaultImportQueue<Block>, sp_consensus::Error>
where
	Block: BlockT,
	C::Api: BlockBuilderApi<Block> + AuraApi<Block, AuthorityId<P>> + ApiExt<Block>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>,
	I: BlockImport<Block, Error = ConsensusError> + Send + Sync + 'static,
	P: Pair + 'static,
	P::Public: Codec + Debug,
	P::Signature: Codec,
	S: sp_core::traits::SpawnEssentialNamed,
	CIDP: CurrentSlotProvider
		+ CreateInherentDataProviders<Block, (Slot, <ID as InherentDigest>::Value)>
		+ Sync
		+ Send
		+ 'static,
	ID: InherentDigest + Send + Sync + 'static,
{
	let verifier = AuraVerifier::<_, P, _, _, ID>::new(
		client,
		create_inherent_data_providers,
		check_for_equivocation,
		telemetry,
		compatibility_mode,
	);

	Ok(BasicQueue::new(verifier, Box::new(block_import), justification_import, spawner, registry))
}

async fn create_inherent_data_provider<CIDP, B: BlockT, ExtraArg>(
	cidp: &CIDP,
	hash: B::Hash,
	extra_arg: ExtraArg,
) -> Result<CIDP::InherentDataProviders, String>
where
	CIDP: CreateInherentDataProviders<B, ExtraArg>,
{
	cidp.create_inherent_data_providers(hash, extra_arg)
		.await
		.map_err(|e| Error::<B>::Client(sp_blockchain::Error::Application(e)).into())
}

async fn create_inherent_data<B: BlockT>(
	provider: &impl InherentDataProvider,
) -> Result<InherentData, Error<B>> {
	Ok(provider.create_inherent_data().await.map_err(Error::<B>::Inherent)?)
}
