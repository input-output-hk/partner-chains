//! Setup code for [`super::command`] which would otherwise bloat that module.
//!
//! Should only be used for benchmarking as it may break in other contexts.

use crate::service::FullClient;

use authority_selection_inherents::ariadne_inherent_data_provider::AriadneInherentDataProvider;
use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionInputs;
use derive_new::new;
use runtime::{AccountId, Balance, BalancesCall, SystemCall};
use sc_cli::Result;
use sc_client_api::BlockBackend;
use sidechain_domain::{
	AuraPublicKey, DParameter, EpochNonce, GrandpaPublicKey, PermissionedCandidateData,
	SidechainPublicKey,
};
use sidechain_runtime as runtime;
use sp_block_rewards::BlockBeneficiaryInherentProvider;
use sp_core::{Encode, Pair};
use sp_inherents::{InherentData, InherentDataProvider};
use sp_keyring::Sr25519Keyring;
use sp_runtime::{OpaqueExtrinsic, SaturatedConversion};
use std::{sync::Arc, time::Duration};

/// Generates extrinsics for the `benchmark overhead` command.
///
/// Note: Should only be used for benchmarking.
#[derive(new)]
pub struct RemarkBuilder {
	client: Arc<FullClient>,
}

impl frame_benchmarking_cli::ExtrinsicBuilder for RemarkBuilder {
	fn pallet(&self) -> &str {
		"system"
	}

	fn extrinsic(&self) -> &str {
		"remark"
	}

	fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
		let acc = Sr25519Keyring::Bob.pair();
		let extrinsic: OpaqueExtrinsic = create_benchmark_extrinsic(
			self.client.as_ref(),
			acc,
			SystemCall::remark { remark: vec![] }.into(),
			nonce,
		)
		.into();

		Ok(extrinsic)
	}
}

/// Generates `Balances::TransferKeepAlive` extrinsics for the benchmarks.
///
/// Note: Should only be used for benchmarking.
#[derive(new)]
pub struct TransferKeepAliveBuilder {
	client: Arc<FullClient>,
	dest: AccountId,
	value: Balance,
}

impl frame_benchmarking_cli::ExtrinsicBuilder for TransferKeepAliveBuilder {
	fn pallet(&self) -> &str {
		"balances"
	}

	fn extrinsic(&self) -> &str {
		"transfer_keep_alive"
	}

	fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
		let acc = Sr25519Keyring::Bob.pair();
		let extrinsic: OpaqueExtrinsic = create_benchmark_extrinsic(
			self.client.as_ref(),
			acc,
			BalancesCall::transfer_keep_alive { dest: self.dest.clone().into(), value: self.value }
				.into(),
			nonce,
		)
		.into();

		Ok(extrinsic)
	}
}

/// Create a transaction using the given `call`.
///
/// Note: Should only be used for benchmarking.
pub fn create_benchmark_extrinsic(
	client: &FullClient,
	sender: sp_core::sr25519::Pair,
	call: runtime::RuntimeCall,
	nonce: u32,
) -> runtime::UncheckedExtrinsic {
	let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
	let best_hash = client.chain_info().best_hash;
	let best_block = client.chain_info().best_number;

	let period = runtime::BlockHashCount::get()
		.checked_next_power_of_two()
		.map(|c| c / 2)
		.unwrap_or(2) as u64;
	let extra: runtime::SignedExtra = (
		frame_system::CheckNonZeroSender::<runtime::Runtime>::new(),
		frame_system::CheckSpecVersion::<runtime::Runtime>::new(),
		frame_system::CheckTxVersion::<runtime::Runtime>::new(),
		frame_system::CheckGenesis::<runtime::Runtime>::new(),
		frame_system::CheckEra::<runtime::Runtime>::from(sp_runtime::generic::Era::mortal(
			period,
			best_block.saturated_into(),
		)),
		frame_system::CheckNonce::<runtime::Runtime>::from(nonce),
		frame_system::CheckWeight::<runtime::Runtime>::new(),
		pallet_transaction_payment::ChargeTransactionPayment::<runtime::Runtime>::from(0),
	);

	let raw_payload = runtime::SignedPayload::from_raw(
		call.clone(),
		extra.clone(),
		(
			(),
			runtime::VERSION.spec_version,
			runtime::VERSION.transaction_version,
			genesis_hash,
			best_hash,
			(),
			(),
			(),
		),
	);
	let signature = raw_payload.using_encoded(|e| sender.sign(e));

	runtime::UncheckedExtrinsic::new_signed(
		call.clone(),
		sp_runtime::AccountId32::from(sender.public()).into(),
		runtime::Signature::Sr25519(signature),
		extra.clone(),
	)
}

const DUMMY_EPOCH_NONCE: &[u8] = &[1u8, 2u8, 3u8];

pub type BeneficiaryId = sidechain_domain::byte_string::SizedByteString<32>;
/// Generates inherent data for the `benchmark overhead` command.
///
/// Note: Should only be used for benchmarking.
pub fn inherent_benchmark_data() -> Result<InherentData> {
	let mut inherent_data = InherentData::new();
	let d = Duration::from_millis(0);
	let timestamp = sp_timestamp::InherentDataProvider::new(d.into());

	let permissioned_candidates = (1u8..=100)
		.map(|i| {
			let sidechain_public_key = SidechainPublicKey(vec![i; 32]);
			let aura_public_key = AuraPublicKey(vec![i; 32]);
			let grandpa_public_key = GrandpaPublicKey(vec![i; 32]);
			PermissionedCandidateData { sidechain_public_key, aura_public_key, grandpa_public_key }
		})
		.collect();

	let ariadne_inherent_data_provider = AriadneInherentDataProvider {
		data: Some(AuthoritySelectionInputs {
			d_parameter: DParameter {
				num_permissioned_candidates: 1,
				num_registered_candidates: 0,
			},
			permissioned_candidates,
			registered_candidates: vec![],
			epoch_nonce: EpochNonce(DUMMY_EPOCH_NONCE.to_vec()),
		}),
	};
	let block_beneficiary_provider =
		BlockBeneficiaryInherentProvider::<BeneficiaryId>::from_env("SIDECHAIN_BLOCK_BENEFICIARY")
			.map_err(|err| sc_cli::Error::Application(err.into()))?;
	futures::executor::block_on(async {
		timestamp.provide_inherent_data(&mut inherent_data).await?;
		ariadne_inherent_data_provider.provide_inherent_data(&mut inherent_data).await?;
		block_beneficiary_provider.provide_inherent_data(&mut inherent_data).await?;
		Ok(())
	})
	.map_err(|e: sp_inherents::Error| format!("creating inherent data: {:?}", e))?;
	Ok(inherent_data)
}
