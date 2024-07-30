use crate::chain_spec::AuthorityKeys;
use crate::chain_spec::ChainSpec;
use crate::main_chain_follower::DataSources;
use async_trait::async_trait;
use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionInputs;
use chain_params::SidechainParams;
use epoch_derivation::MainchainEpochConfig;
use epoch_derivation::{EpochConfig, MainchainEpochDerivation};
use sidechain_domain::McEpochNumber;
use sidechain_domain::ScEpochNumber;
use sidechain_runtime::opaque::SessionKeys;
use sidechain_runtime::CrossChainPublic;
use sidechain_runtime::Runtime;
use sp_api::ApiError;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::offchain::Timestamp;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::traits::Zero;
use sp_session_validator_management::{MainChainScripts, SessionValidatorManagementApi};
use sp_sidechain::GetSidechainParams;
use std::fs;
use std::ops::RangeInclusive;
use std::result::Result;
use std::sync::Arc;
use tokio::time::{self, Duration};

const RETRY_SLEEP_DURATION: Duration = Duration::from_secs(20);
pub const GENERATED_CHAIN_SPEC_FILE_NAME: &str = "generated_chain_spec.json";

#[async_trait]
pub trait Sleeper {
	async fn sleep(&self, duration: Duration);
}

pub struct SleeperLive;

#[async_trait]
impl Sleeper for SleeperLive {
	async fn sleep(&self, duration: Duration) {
		time::sleep(duration).await;
	}
}

pub trait EpochTimer {
	fn get_current_epoch(&self) -> Result<McEpochNumber, String>;
}

pub struct EpochTimerLive {
	mainchain_epoch_config: MainchainEpochConfig,
}

impl EpochTimer for EpochTimerLive {
	fn get_current_epoch(&self) -> Result<McEpochNumber, String> {
		self.mainchain_epoch_config
			.timestamp_to_mainchain_epoch(get_current_timestamp())
			.map_err(|err| format!("Failed to get mainchain epoch: {:?}", err))
	}
}

fn get_current_timestamp() -> Timestamp {
	use std::time::SystemTime;

	let now = SystemTime::now();
	let duration = now
		.duration_since(SystemTime::UNIX_EPOCH)
		.expect("Current time is always after unix epoch; qed");

	Timestamp::from_unix_millis(duration.as_millis() as u64)
}

async fn get_initial_authorities<C, B>(
	client: Arc<C>,
	data_sources: &DataSources,
	epoch_range: RangeInclusive<u32>,
) -> Result<Vec<(CrossChainPublic, SessionKeys)>, ApiError>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: GetSidechainParams<B, SidechainParams>,
	C::Api: SessionValidatorManagementApi<
		B,
		SessionKeys,
		CrossChainPublic,
		AuthoritySelectionInputs,
		ScEpochNumber,
	>,
	C: HeaderBackend<B>,
{
	let api = client.runtime_api();
	let block_hash = client.info().best_hash;

	let mut candidates = None;

	let minimum_mc_epoch = *epoch_range.start();

	// Starting from the `minimum_mc_epoch` and ascending, try to find a candidates list
	for epoch_number in epoch_range {
		candidates = get_candidates::<C, B>(&api, block_hash, data_sources, epoch_number).await?;

		if candidates.is_some() {
			// If we've successfully found a candidates list this loop and this is the first loop...
			if epoch_number == minimum_mc_epoch {
				// Look backwards to see if the candidates list we found truly is the earliest occurrence of a
				// candidates list. If it's not, continue backtracking until we find the first list.
				for backtrack_epoch_number in (0..minimum_mc_epoch).rev() {
					match get_candidates::<C, B>(
						&api,
						block_hash,
						data_sources,
						backtrack_epoch_number,
					)
					.await?
					{
						Some(cs) => candidates = Some(cs),
						// If there's no value here, that means we've found the earliest candidates list
						None => break,
					}
				}
			}
			break;
		}
	}

	Ok(candidates.unwrap_or(vec![]))
}

async fn get_candidates<C, B>(
	api: &sp_api::ApiRef<'_, <C as ProvideRuntimeApi<B>>::Api>,
	block_hash: B::Hash,
	data_sources: &DataSources,
	epoch_no: u32,
) -> Result<Option<Vec<(CrossChainPublic, SessionKeys)>>, ApiError>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: GetSidechainParams<B, SidechainParams>,
	C::Api: SessionValidatorManagementApi<
		B,
		SessionKeys,
		CrossChainPublic,
		AuthoritySelectionInputs,
		ScEpochNumber,
	>,
	C: HeaderBackend<B>,
{
	let scripts = api.get_main_chain_scripts(block_hash)?;
	let authority_selection_inputs = match AuthoritySelectionInputs::from_mc_data(
		data_sources.candidate.as_ref(),
		McEpochNumber(epoch_no),
		scripts,
	)
	.await
	.ok()
	{
		Some(asi) => asi,
		None => return Ok(None),
	};

	api.calculate_committee(
		block_hash,
		authority_selection_inputs,
		sidechain_domain::ScEpochNumber::zero(),
	)
}

async fn wait_until_initial_authorities_ready<C, B, S, E>(
	client: Arc<C>,
	data_sources: &DataSources,
	min: McEpochNumber,
	sleeper: S,
	epoch_timer: E,
) -> Result<Vec<(CrossChainPublic, SessionKeys)>, String>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: GetSidechainParams<B, SidechainParams>,
	C::Api: SessionValidatorManagementApi<
		B,
		SessionKeys,
		CrossChainPublic,
		AuthoritySelectionInputs,
		ScEpochNumber,
	>,
	C: HeaderBackend<B>,
	S: Sleeper + Send + Sync + 'static,
	E: EpochTimer,
{
	let mut start_point = min.0;

	let initial_authorities: Vec<(CrossChainPublic, SessionKeys)> = loop {
		let max = epoch_timer.get_current_epoch()?;

		if start_point > max.0 {
			start_point = max.0
		};

		match get_initial_authorities(client.clone(), data_sources, start_point..=max.0).await {
			Ok(authorities) if !authorities.is_empty() => {
				log::info!(
					"Chain initialization: {} authorities are selected as the initial committee",
					authorities.len()
				);
				break authorities;
			},
			Ok(_) => {
				log::info!("Initial authorities list is empty, retrying in 20 seconds");
				start_point = max.0;
				sleeper.sleep(RETRY_SLEEP_DURATION).await;
				continue;
			},
			Err(e) => {
				log::error!("Failed to get initial authorities, reason: {}", e);
				return Err(e.to_string());
			},
		};
	};

	Ok(initial_authorities)
}

pub async fn get_initial_authorities_with_waits<B, C, S, E>(
	client: Arc<C>,
	data_sources: &DataSources,
	minimum_mc_epoch: McEpochNumber,
	sleeper: S,
	epoch_timer: E,
) -> Result<Vec<AuthorityKeys>, String>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: GetSidechainParams<B, SidechainParams>,
	C::Api: SessionValidatorManagementApi<
		B,
		SessionKeys,
		CrossChainPublic,
		AuthoritySelectionInputs,
		ScEpochNumber,
	>,
	C: HeaderBackend<B>,
	S: Sleeper + Send + Sync + 'static,
	E: EpochTimer,
{
	let initial_authorities = wait_until_initial_authorities_ready(
		client.clone(),
		data_sources,
		minimum_mc_epoch,
		sleeper,
		epoch_timer,
	)
	.await?;

	let authority_keys: Vec<AuthorityKeys> = initial_authorities
		.into_iter()
		.map(|(pk, session_keys)| AuthorityKeys {
			session: SessionKeys { aura: session_keys.aura, grandpa: session_keys.grandpa },
			cross_chain: pk,
		})
		.collect();

	Ok(authority_keys)
}

pub async fn run<B, C, S>(
	client: Arc<C>,
	data_sources: &DataSources,
	epoch_config: &EpochConfig,
	config: &sc_service::Configuration,
	sleeper: S,
) -> Result<Box<dyn sc_service::ChainSpec>, String>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: GetSidechainParams<B, SidechainParams>,
	C::Api: SessionValidatorManagementApi<
		B,
		SessionKeys,
		CrossChainPublic,
		AuthoritySelectionInputs,
		ScEpochNumber,
	>,
	C: HeaderBackend<B>,
	S: Sleeper + Send + Sync + 'static,
{
	cleanup_persistent_storage(config)?;

	let minimum_mc_epoch = McEpochNumber(
		std::env::var("MINIMUM_MC_EPOCH")
			.ok()
			.and_then(|val| val.parse::<u32>().ok())
			.unwrap_or(epoch_config.mc.first_epoch_number),
	);

	let authority_keys = get_initial_authorities_with_waits(
		client.clone(),
		data_sources,
		minimum_mc_epoch,
		sleeper,
		EpochTimerLive { mainchain_epoch_config: epoch_config.mc.clone() },
	)
	.await?;

	let genesis_main_chain_scripts = client
		.runtime_api()
		.get_main_chain_scripts(client.info().best_hash)
		.map_err(|e| e.to_string())?;
	set_initial_authorities(
		authority_keys,
		genesis_main_chain_scripts,
		config.chain_spec.cloned_box(),
	)
}

pub(crate) fn set_initial_authorities(
	initial_authorities: Vec<AuthorityKeys>,
	chain_spec_main_chain_scripts: MainChainScripts,
	original_chain_spec: Box<dyn sc_service::ChainSpec>,
) -> Result<Box<dyn sc_service::ChainSpec>, String> {
	let mut modified_chain_spec = original_chain_spec.cloned_box();
	let mut storage = modified_chain_spec.build_storage()?;
	insert_into_storage(initial_authorities, chain_spec_main_chain_scripts, &mut storage)?;
	modified_chain_spec.set_storage(storage);
	Ok(modified_chain_spec)
}

pub(crate) fn insert_into_storage(
	initial_authorities: Vec<AuthorityKeys>,
	chain_spec_main_chain_scripts: MainChainScripts,
	original_chain_spec_storage: &mut sp_core::storage::Storage,
) -> Result<(), String> {
	use sp_runtime::BuildStorage;
	let mut overlay_storage = sp_core::storage::Storage::default();

	let authorities_for_session_validator_mgmt_pallet = initial_authorities
		.iter()
		.map(|x| (x.cross_chain.clone(), x.session.clone()))
		.collect::<Vec<_>>();
	pallet_session_validator_management::GenesisConfig::<Runtime> {
		initial_authorities: authorities_for_session_validator_mgmt_pallet,
		main_chain_scripts: chain_spec_main_chain_scripts,
	}
	.assimilate_storage(&mut overlay_storage)?;

	let authorities_for_session_pallet = initial_authorities
		.iter()
		.map(|x| (x.cross_chain.clone().into(), x.session.clone()))
		.collect::<Vec<_>>();
	pallet_partner_chains_session::GenesisConfig::<Runtime> {
		initial_validators: authorities_for_session_pallet,
	}
	.assimilate_storage(&mut overlay_storage)?;
	original_chain_spec_storage.top.extend(overlay_storage.top);
	Ok(())
}

pub fn save_spec(spec: Box<dyn sc_service::ChainSpec>, file_name: &str) -> Result<(), String> {
	spec.as_json(false)
		.map_err(|e| e.to_string())
		.and_then(|spec_json| fs::write(file_name, spec_json.as_bytes()).map_err(|e| e.to_string()))
}

pub fn read_spec(file_name: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
	Ok(Box::new(ChainSpec::from_json_file(std::path::PathBuf::from(file_name))?))
}

// Delete the folder with persistent storage.
// During initialization, a `chain_init_client` is created to enable runtime API calls, which as
// a side effect, generates a RocksDB database populated with genesis data from the initial chain_spec.
// To prevent genesis data mismatch errors and ensure successful node startup, it's essential to remove
// this folder, thereby cleaning up the storage created by `chain_init_client`.
fn cleanup_persistent_storage(config: &sc_service::Configuration) -> Result<(), String> {
	let db_path = config
		.database
		.path()
		.and_then(|p| p.parent())
		.ok_or("Database folder not found")?;
	std::fs::remove_dir_all(db_path)
		.map_err(|err| format!("Failed to remove the base directory: {:?}", err))
}
