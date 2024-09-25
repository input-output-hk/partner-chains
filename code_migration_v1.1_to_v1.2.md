# Code migration guide for partner-chains v1.1.0 to v1.2.0

## partner-chains-node-commands
Code supporting `generate-signatures`, `get-ariadne-parameters` and `get-registration-status` commands has been moved from the reference node to library crates of `partner-chains`.
Moving this code to library crates will make it less code to update in the future.
Any node has to recognize these commands in order to be a valid partner chain node.

1. Add `partner-chains-node-commands` crate to your dependencies in *Cargo.toml*.

2. Modify `Subcommand` enum (in reference implementation it is in *cli.rs* file):
```rust
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	// ... other subcommands
	#[clap(flatten)]
	PartnerChains(PartnerChainsSubcommand<YourSidechainParams>),
	// ... other subcommands
}
```
, where `YourSidechainParams` is the struct that holds the parameters of your partner chains. Reference implementation uses `chain_params::SidechainParams` struct.

3. Wire all the partner chains commands in the `run` function of your node. For example:
```rust
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		// ... other commmands
		Some(Subcommand::PartnerChains(cmd)) => {
			let make_dependencies = |config| {
				let components = service::new_partial(&config)?;
				Ok((components.client, components.task_manager, components.other.3.candidate))
			};
			partner_chains_node_commands::run(&cli, make_dependencies, cmd.clone())
		},
		/// ... other commands
```

4. Cleanup *cli.rs* and *mod.rs* files in case you node had copy-paste implementation of these commands.

# polkadot-sdk

partner-chains toolkit has migrated from custom fork of polkadot-sdk to paritytech fork of polkadot-sdk.
This changes the structure of crates used by partner-chains toolkit.

Please update your dependencies to reflect this change and depend only on the paritytech fork.
Replace `git = "https://github.com/input-output-hk/polkadot-sdk.git", tag = "partnerchains-stable2407"` with `"https://github.com/paritytech/polkadot-sdk.git", tag = "polkadot-stable2407-2"` in your cargo files.

## sc-partner-chains-consensus-aura

input-output-hk fork of polkadot used modified `sc-consensus-aura` crate to inject slot in inherent data providers and to have access to inherent digest (Cardano stable block hash).
These modifications are moved to partner-chains crate `sc-partner-chains-consensus-aura` that depends on unmodified `sc-consensus-aura` crate from paritytech/polkadot-sdk.

Add `sc-partner-chains-consensus-aura` to your dependencies in `Cargo.toml`.

Code changes related wiring this change are (*service.rs* file in reference node implementation):

1. Replace `use sp_partner_chains_consensus_aura::CurrentSlotProvider;` with `use sc_consensus_aura::CurrentSlotProvider;`.

2. Replace `sc_consensus_aura::import_queue` with `sc_partner_chains_consensus_aura::import_queue`.

3. Replace `sc_consensus_aura::start_aura` with `sc_partner_chains_consensus_aura::start_aura`.

4. Use custom `BlockProposerFactory` from `sc-partner-chains-consensus-aura` crate:
```rust
let basic_authorship_proposer_factory = sc_basic_authorship::ProposerFactory::new(
	task_manager.spawn_handle(),
	client.clone(),
	transaction_pool.clone(),
	prometheus_registry.as_ref(),
	telemetry.as_ref().map(|x| x.handle()),
);
let proposer_factory: PartnerChainsProposerFactory<_, _, McHashInherentDigest> =
	PartnerChainsProposerFactory::new(basic_authorship_proposer_factory);
```

## pallet-partner-chains-session

This pallet was moved from polkadot-sdk to partner-chains, please update your cargo files.
Also add `pallet-session-runtime-stub` from partner-chains to your runtime dependencies.
Add `pallet-session` from polkadot-sdk as well to runtime and node dependencies.

## runtime code

It is required to add `pallet-session` `Runtime` configuration to your `construct_runtime!` macro.
What is more, currently it has to be stubbed implementation, because of how `pallet-partner-chains-session` is implemented and wired in.
To obtain this stub implementation use:
```rust
pallet_session_runtime_stub::impl_pallet_session_config!(Runtime);
```
Then add: `PolkadotSession:pallet_session` in the `construct_runtime!` macro.

Increase the `spec_version`, so it will be possible to upgrade the runtime.

## sc-cli

`Runner` has been reverted to pairtytech version, this requires changes in the node code,
because the vanilla polkadot-sdk version doesn't support async in a way our custom code did.

Required change is in *service.rs* file, where the main chain follower has to be created in a blocking way:
```rust
let data_sources = task::block_in_place(|| {
	config.tokio_handle.block_on(
		crate::main_chain_follower::create_cached_main_chain_follower_data_sources(
			mc_follower_metrics.clone(),
		),
	)
})?;
```

`pub fn new_partial` is not async anymore, please follow the change in your implementation.

Please remove `async move` and `.await` in the `run` function of your node in places where compiler complains.
For example replace:
```rust
Some(Subcommand::CheckBlock(cmd)) => {
	let runner = cli.create_runner(cmd)?;
	runner.async_run(|config| async move {
		let PartialComponents { client, task_manager, import_queue, .. } =
			service::new_partial(&config).await?;
		Ok((cmd.run(client, import_queue), task_manager))
	})
},
```
with:
```rust
Some(Subcommand::CheckBlock(cmd)) => {
	let runner = cli.create_runner(cmd)?;
	runner.async_run(|config| {
		let PartialComponents { client, task_manager, import_queue, .. } =
			service::new_partial(&config)?;
		Ok((cmd.run(client, import_queue), task_manager))
	})
},
```

## chain-init feature

Code that was present in `node` crate related to `USE_CHAIN_INIT` feature has been removed.
The only supported way to obtain the chain-spec is through partner-chains-cli.
Please cleanup your node if it contained copy-pasted code for it.

# Observation of native token illiquid supply address

This feature is not complete, please do not follow changes in partner-chains reference runtime and node.
Please do not wire it in your node code nor use the "native-token" feature in the main chain follower.
