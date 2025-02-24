//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use crate::inherent_data::{CreateInherentDataConfig, ProposalCIDP, VerifierCIDP};
use crate::main_chain_follower::DataSources;
use crate::rpc::GrandpaDeps;
use db_sync_follower::metrics::register_metrics_warn_errors;
use db_sync_follower::metrics::McFollowerMetrics;
use sc_client_api::BlockBackend;
use sc_consensus_aura::{ImportQueueParams, SlotProportion, StartAuraParams};
use sc_consensus_grandpa::SharedVoterState;
pub use sc_executor::WasmExecutor;
use sc_partner_chains_consensus_aura::import_queue as partner_chains_aura_import_queue;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager, WarpSyncConfig};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sidechain_domain::mainchain_epoch::MainchainEpochConfig;
use sidechain_mc_hash::McHashInherentDigest;
use sidechain_runtime::{self, opaque::Block, RuntimeApi};
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sp_partner_chains_consensus_aura::block_proposal::PartnerChainsProposerFactory;
use sp_runtime::traits::Block as BlockT;
use std::{sync::Arc, time::Duration};
use time_source::SystemTimeSource;
use tokio::task;

type HostFunctions = sp_io::SubstrateHostFunctions;

pub(crate) type FullClient =
	sc_service::TFullClient<Block, RuntimeApi, WasmExecutor<HostFunctions>>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

#[allow(clippy::type_complexity)]
pub fn new_partial(
	config: &Configuration,
) -> Result<
	sc_service::PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		sc_consensus::DefaultImportQueue<Block>,
		sc_transaction_pool::TransactionPoolHandle<Block, FullClient>,
		(
			sc_consensus_grandpa::GrandpaBlockImport<
				FullBackend,
				Block,
				FullClient,
				FullSelectChain,
			>,
			sc_consensus_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
			Option<Telemetry>,
			DataSources,
			Option<McFollowerMetrics>,
		),
	>,
	ServiceError,
> {
	let mc_follower_metrics = register_metrics_warn_errors(config.prometheus_registry());
	let data_sources = task::block_in_place(|| {
		config.tokio_handle.block_on(
			crate::main_chain_follower::create_cached_main_chain_follower_data_sources(
				mc_follower_metrics.clone(),
			),
		)
	})?;

	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = sc_service::new_wasm_executor(&config.executor);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = Arc::from(
		sc_transaction_pool::Builder::new(
			task_manager.spawn_essential_handle(),
			client.clone(),
			config.role.is_authority().into(),
		)
		.with_options(config.transaction_pool.clone())
		.with_prometheus(config.prometheus_registry())
		.build(),
	);

	let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
		client.clone(),
		GRANDPA_JUSTIFICATION_PERIOD,
		&client,
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	let sc_slot_config = sidechain_slots::runtime_api_client::slot_config(&*client)
		.map_err(sp_blockchain::Error::from)?;

	let time_source = Arc::new(SystemTimeSource);
	let epoch_config = MainchainEpochConfig::read_from_env()
		.map_err(|err| ServiceError::Application(err.into()))?;
	let inherent_config = CreateInherentDataConfig::new(epoch_config, sc_slot_config, time_source);

	let import_queue = partner_chains_aura_import_queue::import_queue::<
		AuraPair,
		_,
		_,
		_,
		_,
		_,
		McHashInherentDigest,
	>(ImportQueueParams {
		block_import: grandpa_block_import.clone(),
		justification_import: Some(Box::new(grandpa_block_import.clone())),
		client: client.clone(),
		create_inherent_data_providers: VerifierCIDP::new(
			inherent_config,
			client.clone(),
			data_sources.mc_hash.clone(),
			data_sources.authority_selection.clone(),
			data_sources.native_token.clone(),
			data_sources.stake_distribution.clone(),
		),
		spawner: &task_manager.spawn_essential_handle(),
		registry: config.prometheus_registry(),
		check_for_equivocation: Default::default(),
		telemetry: telemetry.as_ref().map(|x| x.handle()),
		compatibility_mode: Default::default(),
	})?;

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (grandpa_block_import, grandpa_link, telemetry, data_sources, mc_follower_metrics),
	})
}

pub async fn new_full<Network: sc_network::NetworkBackend<Block, <Block as BlockT>::Hash>>(
	config: Configuration,
) -> Result<TaskManager, ServiceError> {
	if let Some(git_hash) = std::option_env!("EARTHLY_GIT_HASH") {
		log::info!("🌱 Running version: {}", git_hash);
	}

	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (block_import, grandpa_link, mut telemetry, data_sources, _),
	} = new_partial(&config)?;

	let metrics = Network::register_notification_metrics(config.prometheus_registry());
	let mut net_config = sc_network::config::FullNetworkConfiguration::<_, _, Network>::new(
		&config.network,
		config.prometheus_registry().cloned(),
	);

	let grandpa_protocol_name = sc_consensus_grandpa::protocol_standard_name(
		&client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
		&config.chain_spec,
	);
	let peer_store_handle = net_config.peer_store_handle();
	let (grandpa_protocol_config, grandpa_notification_service) =
		sc_consensus_grandpa::grandpa_peers_set_config::<_, Network>(
			grandpa_protocol_name.clone(),
			metrics.clone(),
			Arc::clone(&peer_store_handle),
		);
	net_config.add_notification_protocol(grandpa_protocol_config);

	let warp_sync = Arc::new(sc_consensus_grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		grandpa_link.shared_authority_set().clone(),
		Vec::default(),
	));

	let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_config: Some(WarpSyncConfig::WithProvider(warp_sync)),
			block_relay: None,
			metrics,
		})?;

	let role = config.role;
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks: Option<()> = None;
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();
	let shared_voter_state = SharedVoterState::empty();

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let backend = backend.clone();
		let shared_voter_state = shared_voter_state.clone();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let justification_stream = grandpa_link.justification_stream();
		let main_chain_follower_data_sources = data_sources.clone();

		move |subscription_executor| {
			let grandpa = GrandpaDeps {
				shared_voter_state: shared_voter_state.clone(),
				shared_authority_set: shared_authority_set.clone(),
				justification_stream: justification_stream.clone(),
				subscription_executor,
				finality_provider: sc_consensus_grandpa::FinalityProofProvider::new_for_service(
					backend.clone(),
					Some(shared_authority_set.clone()),
				),
			};
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				grandpa,
				main_chain_follower_data_sources: main_chain_follower_data_sources.clone(),
				time_source: Arc::new(SystemTimeSource),
			};
			crate::rpc::create_full(deps).map_err(Into::into)
		}
	};

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network: network.clone(),
		client: client.clone(),
		keystore: keystore_container.keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder: Box::new(rpc_extensions_builder),
		backend,
		system_rpc_tx,
		tx_handler_controller,
		sync_service: sync_service.clone(),
		config,
		telemetry: telemetry.as_mut(),
	})?;

	if role.is_authority() {
		let basic_authorship_proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);
		let proposer_factory: PartnerChainsProposerFactory<_, _, McHashInherentDigest> =
			PartnerChainsProposerFactory::new(basic_authorship_proposer_factory);

		let sc_slot_config = sidechain_slots::runtime_api_client::slot_config(&*client)
			.map_err(sp_blockchain::Error::from)?;
		let time_source = Arc::new(SystemTimeSource);
		let mc_epoch_config = MainchainEpochConfig::read_from_env()
			.map_err(|err| ServiceError::Application(err.into()))?;
		let inherent_config =
			CreateInherentDataConfig::new(mc_epoch_config, sc_slot_config.clone(), time_source);
		let aura = sc_partner_chains_consensus_aura::start_aura::<
			AuraPair,
			_,
			_,
			_,
			_,
			_,
			_,
			_,
			_,
			_,
			_,
			McHashInherentDigest,
		>(StartAuraParams {
			slot_duration: sc_slot_config.slot_duration,
			client: client.clone(),
			select_chain,
			block_import,
			proposer_factory,
			create_inherent_data_providers: ProposalCIDP::new(
				inherent_config,
				client.clone(),
				data_sources.mc_hash.clone(),
				data_sources.authority_selection.clone(),
				data_sources.native_token.clone(),
				data_sources.stake_distribution,
			),
			force_authoring,
			backoff_authoring_blocks,
			keystore: keystore_container.keystore(),
			sync_oracle: sync_service.clone(),
			justification_sync_link: sync_service.clone(),
			block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			compatibility_mode: Default::default(),
		})?;

		// the AURA authoring task is considered essential, i.e. if it
		// fails we take down the service with it.
		task_manager
			.spawn_essential_handle()
			.spawn_blocking("aura", Some("block-authoring"), aura);
	}

	if enable_grandpa {
		// if the node isn't actively participating in consensus then it doesn't
		// need a keystore, regardless of which protocol we use below.
		let keystore = if role.is_authority() { Some(keystore_container.keystore()) } else { None };

		let grandpa_config = sc_consensus_grandpa::Config {
			// FIXME #1578 make this available through chainspec
			gossip_duration: Duration::from_millis(333),
			justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
			name: Some(name),
			observer_enabled: false,
			keystore,
			local_role: role,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			protocol_name: grandpa_protocol_name,
		};

		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_config = sc_consensus_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network,
			sync: Arc::new(sync_service),
			notification_service: grandpa_notification_service,
			voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			sc_consensus_grandpa::run_grandpa_voter(grandpa_config)?,
		);
	}

	network_starter.start_network();
	Ok(task_manager)
}
