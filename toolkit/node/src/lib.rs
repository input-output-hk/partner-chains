pub mod data_source;
pub mod rpc;

use derive_new::new;
use sidechain_domain::mainchain_epoch::MainchainEpochConfig;
use sidechain_slots::*;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

#[derive(Clone, new)]
pub struct PartnerChainsNodeConfig {
	pub mc_epoch_config: MainchainEpochConfig,
	// TODO ETCM-4079 make sure that this struct can be instantiated only if sidechain epoch duration is divisible by slot_duration
	pub sc_slot_config: ScSlotConfig,
}

impl PartnerChainsNodeConfig {
	pub fn new_from_env<B, C>(client: &C) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
	where
		B: BlockT,
		C: sp_api::ProvideRuntimeApi<B>,
		C: HeaderBackend<B>,
		C::Api: SlotApi<B>,
	{
		let sc_slot_config = sidechain_slots::runtime_api_client::slot_config(client)?;
		let mc_epoch_config = MainchainEpochConfig::read_from_env()?;

		Ok(Self::new(mc_epoch_config, sc_slot_config))
	}
}
