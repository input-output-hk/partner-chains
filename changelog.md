# Changelog

This changelog is based on [Keep A Changelog](https://keepachangelog.com/en/1.1.0).

# Unreleased

* Added extra constant burn fee in `pallet-address-association` to discourage attacks on pallet storage.
* Wizards don't require `generate-keys` for `prepare-configuration`. Altered recommended order of `create-chain-spec` and `setup-main-chain-state`.

## Changed

* `pallet-block-producer-metadata` is updated with a configurable fee for inserting the metadata, to make attacks on unbounded storage economically infeasible

## Added

* `delete_metadata` extrinsic in `pallet-block-producer-metadata`

## Fixed

* `smart-contracts` governance actions were failing due too redundant signature when initiated by non-governance wallet
* Wizards using 'sidechain' in command line parameters are changed to use 'partner-chain' instead

## Removed

# v1.7.0

## Changed

* `partner-chains-db-sync-data-sources` and `partner-chains-mock-data-sources` crates now exports all its public members from the root
* `partner-chains-db-sync-data-sources` crate now exports all its public members from the root
* `smart-contracts` commands can accept parameter to configure Ogmios requests timeout
* `prepare-configuration` and `create-chain-spec` wizards are updated to setup `governedMap.MainChainScripts` in the chain-spec file.
* `setup-main-chain-state` wizard uses Ogmios and `offchain` crate for getting the current D-parameter and Permissioned Candidates instead of invoking `<node-executable> ariadne-parameters` command.
* `prepare-configuration` wizard suggests payment signing key hash as governance authority if there is no value in chain config stored so far.
* Automatically create required index on `tx_out` table `address` column. Constructor signatures have changed - this change is not source compatible.
* BREAKING: Wizards are not generating keys nor looking for them in `<base_path>/chains/partner_chains_template` but use `<base_path>` instead.
This change requires users that generated keystores and network key using previous versions of the wizards to move their keys two directories above.
* Wizards are adjusted to use multiple governance authorities from the chain governance initialization through setting up Ariadne parameters
* Added implementation of `FindAccountFromAuthorIndex` trait for `pallet_partner_chains_session`.
* Unified `*toml` files package data
* Renamed `sidechain-runtime` and `sidechain-node` to `partner-chains-demo-runtime` and `partner-chains-demo-node`
respectively. Moved both crates to `demo/` directory.
* Renamed `db-sync-follower` and `main-chain-follower-mock` crates to `partner-chains-db-sync-data-sources` and
`partner-chains-mock-data-sources`. Environment variable `MAIN_CHAIN_FOLLOWER_MOCK_REGISTRATIONS_FILE` used by
the mock data source has been changed to `MOCK_REGISTRATIONS_FILE` to match. Prometheus metrics
`mc_follower_method_time_elapsed` and `mc_follower_method_call_count` were also renamed to
`partner_chains_data_source_method_time_elapsed` and `partner_chains_data_source_method_call_count` respectively.
* Default `smart-contracts` timeout from 2 minutes to 5 minutes
* Update polkadot-sdk to polkadot-stable2503.
* `McHashInherentDataProvider` now also exposes the Cardano block hash referenced by the previous block,
if any. It can be accessed through `previous_mc_hash` function.
* `NativeTokenManagementInherentDataProvider` now expects to be passed the previous main chain reference
block hash on construction instead of retrieving it by itself. Use `McHashInherentDataProvider::previous_mc_hash`
to provide it in your IDP stack.
* Introduced customization of root origin for few pallets via `MainChainScriptsOrigin` trait
* Made `mock` module of `pallet-session-validator-management` private
* Updated dependecies
* Updated polkadot-sdk to polkadot-stable2503-2
* Deprecated the `GetSidechainStatus` runtime API in `sp-sidechain` crate. Code
that needs data that it served should define its own runtime API instead.
* Updated polkadot-sdk to polkadot-stable2503-5
* Updated partner-chains-smart-contracts (raw-scripts) dependency to v7.2.2.
This new version changes Version Oracle Validator script and is not compatible with the previous version.

## Removed

* [**BREAKING CHANGE**] Obsolete `block-rewards` pallet and its companion primitives crate. Block authorship information can be tracked
using the `block-production-log` pallet instead.
* Crate `pallet-session-runtime-stub` which provided stub config for Substrate's `pallet-session` when using `pallet-partner-chains-session`.
Its functionality was merged into `pallet-partner-chains-session` under the feature `pallet-session-compat`.
* Crate `session-manager`. Its functionality was merged into `pallet-session-validator-management` under
the feature `pallet-session-compat`.
* `TryFrom<&serde_json::Value> for Datum` and `From<&Datum> for serde_json::Value` instances from `plutus`.
* `ATMSPlainAggregatePubKey`, `ValidatorHash` and `SidechainPublicKeysSorted` types from `domain`.
* `SidechainApi` trait from `sp-sidechain` and its return type `SidechainStatus`. Code that uses it should directly use
the APIs that were grouped into this trait or ideally define its own runtime API instead (see deprecation of `GetSidechainStatus`).
* Trait `PartnerChainRuntimeBindings` and merged into `PartnerChainRuntime` trait.
* Trait `CreateChainSpecRuntimeBindings` from `partner-chains-cli`, and substituted with `PartnerChainRuntime` trait.

## Fixed

* `prepare-configuration` wizard now updates existing `chain_parameters.genesis_utxo` field in `pc-chain-config.json`
* MC Hash inherent data provider will not propose older MC state than one already present in the ledger
* `governance init` when genesis utxo had a script attached, then transaction fee was sometimes calculated incorrectly
* [SECURITY FIX] Vulnerability of multiple crates, where a malicious block producing node could put multiple copies
of the inherent in the block. This was because Substrate only checks whether an inherent is valid and doesn't ensure
its uniqueness. This issue was fixed by including checks within the inherents themselves. Affected pallets that were
patched are: `session-validator-management`, `block-participation`, `native-token-management`.
**Partner Chain builders should update their pallet versions and run a runtime upgrade as soon as possible.**

## Added
* `pallet-block-producer-fees` - with settings for the rewards payout logic
* `governed-map` new feature that allows setting any arbitrary data to be managed via existing governance mechanism
* `ariadne_v2` selection algorithm that selects committee respecting D-parameter and candidates
weights, but has much less variance, thanks to assigning guaranteed seats to candidates with
expected number of seats greater or equal 1.

### MultiSig support for governance operations

Now, Governance mechanism uses ALeastN Native Script, instead of custom policy implemented as Plutus Script in partner-chains-smart-contracts. This policy doesn't require to set `required_signers` field in the transaction making it more user friendly.

`governance init` and `governance update` now accept multiple key hashes
for the `-g`/`--governance-authority` parameter.
These commands now also require `-t`/`threshold` parameter to set the number
of required signatures.

All the `smart-contracts` sub-commands that require Governance: `governance update`, `upsert-d-parameter`, `upsert-permissioned-candidates`, `reserve init`, `reserve create`, `reserve deposit`, and `reserve handover` will now submit the transaction only if governance is "1 of 1". Otherwise these commands return a transaction CBOR that can be submitted with the new command `assemble-and-submit-tx`. Signatures can be obtained using `sign-tx`.

Procedure of creating transaction to sign is as follows:
* a temporary wallet is generated
* temporary wallet private key is saved to a file
* `--payment-key` transfers required funds to the temporary wallet
* a transaction paid from this temporary wallet is created
* transaction and temporary wallet data are printed to stdout.

`assemble-and-submit-tx` and `sign-tx` are added for unified UX.
Signing and transaction submission can be done in other ways as well.

`governance get-policy` subcommand prints the current Governance Policy.

### Other additions

* `sign-tx` command to `smart-contracts` commands for signing transactions
* `sign-block-producer-metadata` command to `cli-commands` for signing block producer metadata upsert message
* `db-sync-sqlx` crate containing Rust types representing Cardano primitives present in postgres tables populated by Db-Sync

# v1.6.1

## Changed

* Default `smart-contracts` timeout from 2 minutes to 5 minutes

## Fixed

* `governance init` when genesis utxo had a script attached, then transaction fee was sometimes calculated incorrectly

## Removed

* Removed unnecessary transitive dependencies from multiple crates

# v1.6.0

## Changed

* Split MainchainPublicKey to StakePoolPublicKey and StakePublicKey. Some parameters names has been changed as well, so potentially compilation of downstream projects could be broken.
* Update polkadot-sdk to polkadot-stable2412-1.
WARNING: Benchmarking command has been removed, because `frame-benchmarking-cli` crate became GPLv3 without any exception.
* Made Cardano slot duration configurable with default of 1000ms. If your partner chain's main chain is Cardano
mainnet or one of the official testnets, you don't need to change anything. Otherwise, the duration can
be set through `MC__SLOT_DURATION_MILLIS` environment variable.
* e2e-tests: updated python to 3.12 and libs versions.
* Committee member data stored by the Session Validator Management Pallet is now fully generic. To migrate to this version,
define your own `CommitteeMember` type and implement the trait `CommitteeMember` for it. See the `CommitteeMember`
type implemented in `node/runtime/src/lib.rs` for reference using Ariadne.
* Merged functionality of `NativeTokenManagementInherentDataProvider::new_if_pallet_present` into `new`. Use this single constructor from now on.
* `smart-contracts reserve deposit` command parameter `token` has been removed, because it was redundant.

## Fixed

* Failure of `smart-contracts reserve release` command when releasing all tokens in the reserve
* Failure of `smart-contracts reserve handover` command when reserve is empty

## Added

* block-production-log pallet, see it's readme for more details.
* Block participation pallet and inherent data provider, making available data on block producers
  and their delegators. This feature is meant to be used by Partner Chains developers to implement
  block production reward payouts in their own runtimes. See `toolkit/primitives/block-participation/README.md`
  for more information.

# v1.5.1

## Changed

* Default `smart-contracts` timeout from 2 minutes to 5 minutes

## Fixed

* Failure of `smart-contracts reserve release` command when releasing all tokens in the reserve
* Failure of `smart-contracts reserve handover` command when reserve is empty

## Removed

* Removed unnecessary transitive dependencies from multiple crates

# v1.5.0

## Changed
* `smart-contracts reserve release` command parameter `--amount` semantic has changed, it now represent the amount of tokens to release in this command execution
* Replaced custom weights with default substrate weights for few pallets
* Updated to polkadot-stable2409-4 (aka v1.16.4).
* `setup-main-chain-state` command now uses native Rust to upsert the D-Parameter and upsert permissioned candidates
* Changed the `smart-contracts init-governance` command to `smart-contracts governance init`
* smart-contracts commands and offchain tests now use WebSockets implementation of Ogmios client
* Updated to polkadot-stable2409-3 (aka v1.16.3).
* `local-environment` now uses the `partner-chains-node` based container for the smart-contracts setup.
* `partner-chains-cli` separate binary is transformed to a library crated and integrated in `partner-chains-node-commands` library crate.
Every invocation of `partner-chains-cli` should be replaced with `<node> wizards` subcommand of the node built with Partner Chains SDK.
The only other change is that "node executable path" configuration is not present in `partner-chains-cli-resources.json` anymore, because it is not needed anymore.
Code will always invoke "self" executable instead.
Since this change, all functionality of Partner Chains is available in the one executable of the node.
* Added support for extended payment signing and verification keys.
* Renamed file names of the configs used by wizard commands. `partner-chains-cli-resources-config.json` is now
`pc-resources-config.json`, `partner-chains-cli-chain-config.json` is now `pc-chain-config.json`. Rename your
files accordingly if migrating from prior versions.
* e2e tests: config has been changed to use `partner-chains-node` to interact with smart contracts
* e2e tests: test_add_permissioned_candidate and test_remove_permissioned_candidate have been merged into test_upsert_permissioned_candidates, and it's now
setting the permissioned candidates exactly as provided, overriding previous candidates. That means you can no longer remove a single candidate, you need
to provide a whole list if you only want to remove one candidate. Set `"permissioned_candidate": true` in your config for every permissioned candidate on your network
to achieve this.

## Removed

* Separate binary partner-chains-smart-contracts-commands.
* Override artifacts option in `local-environment` (docker image artifact is used).
* Removed the `mock-types` crate.

## Fixed

* Cache returning invalid results when native token MainChainScripts has changed.
* Crash of partner-chain-node smart-contracts command. Logging is now set independently.
* Renamed of argument 'ogmios-host' to 'ogmios-url' in smart-contracts subcommands.

## Added

* Address Associations pallet and signing command
* Command `smart-contracts reserve init`, `create`, `deposit`, `release`, `handover` and `update-settings`
* Command `smart-contracts governance update`
* Ogmios client backed by jsonrpsee `WsClient`
* Input data is now included in the `set` committee inherent in `pallet-session-validator-management`

# v1.4.0

## Changed

* `genesis_utxo` and `registration_utxo` no longer have to have no native tokens.
* Update dependencies containers to cardano-node 10.1.4, db-sync 13.6.0.4, ogmios 6.11.0 and kupo 2.10.0
* Organized Rust sources into two directories: toolkit and node.
* Implemented transaction balancing with CSL in offchain code.
* Update offchain code dependencies: pallas, ulpc and cardano-serialization-lib.
* Updated to partner-chains-smart-contracts v7.0.2
* * chain-params crate that provided SidechainParam is removed, because there are no SidechainParams anymore
* * partner-chains-cli is changed, so prepare-config wizard sets `genesis_utxo` and does not set sidechain parameters
* * pallets are not generic on SidechainParams anymore, they use UtxoId (genesis_utxo) instead
* * This change requires manual migration, because automatic migration of storage in this case is impossible.
Please refer to the migration guide at `docs/developer-guides/migration-guide-1.4.0.md` for detailed
instructions on how to manually upgrade a running chain to 1.4.0.
Do **NOT** perform a normal runtime upgrade, it will break your chain.

## Fixed

* CardanoNetwork bug in `partner-chains-cli`, that would cause the CLI to fail with the mainnet.

## Added

* Added `smart-contracts` command to the node with sub-commands `init-governance`, `get-scripts` and
`upsert-d-parameter`.

# v1.3.0

## Changed

* Added 'deregister' command to partner-chains-cli.
* Made `MainChainScripts` in the native token pallet optional. If they are not set, the inherent data
provider will not query the main chain state or produce inherent data at all.
* ETCM-8366 - native token management pallet can now observe historical transfers when added after the genesis block
* Updated to polkadot-stable2409 (aka v1.16.0).
* * Requires some downstream changes in the node code.
* * See diff of the commit that adds this changelog line for hints.
* * Specific changes will depend on the node implementation.
* Update toolchain to 1.81.0
* Implemented batch queries and caching for the native token observability. Improves performance of the full-sync.
* Added ogmios-client internal library for communication with Ogmios
* Using Ogmios for reading Cardano Network parameters in `partner-chains-cli`, instead of asking user to choose them
* Bugfix: rephrased vague log message when selecting the epoch committee
* Removed the `main-chain-follower-api` completely. Each crate that depended on it now defines its own `*DataSource`
trait, implemented by separate types in `db-sync-follower` and `main-chain-follower-mock` crates. For reference
on how to create these new data sources see `node/src/main_chain_follower.rs` file.
* Added `pallet-session` integration for `pallet-session-validator-management`. Not wired in the node.
* `partner-chains-cli` does not use `cardano-cli` to derive address not to query utxos.
* `partner-chains-cli` does not use `pc-contracts-cli` in `prepare-configuration` wizard, it uses `partner-chains-cardano-offchain` crate instead.
* Update cardano-node to 10.1.2

## Added
* Added `new_if_pallet_present` factory for the native token inherent data provider,
allowing to selectively query main chain state based on runtime version
* Added Largest-First coin selection algorithm.

# 1.2.0

## Changed

* Switched to paritytech/polkadot-sdk polkadot-stable2407-2. No migration is required because of this change.
* Reverted usage of custom Runner that allowed `async_run` with asynchronous initializer.
  Now `Runner` code used is the same as in paritytech/polkadot-sdk.
  This change requires updates in node: `new_partial` cannot be async.
  Run command dispatch looks more like in paritytech/polkadot-sdk.
* bugfix for Mainnet compatibility in the db-sync main-chain follower. Fixes null block_no column decoding problem.
* moved out some cli related code from `node` crate, in order to require less copy-paste in users nodes
* removed USE_CHAIN_INIT code. Migration strategy is to remove copy-pasted and adapted code. It will not compile with vanilla polkadot-sdk, that we plan to use in future.
* moved `pallet-partner-chains-session` from polkadot-sdk fork to this repository. Node uses vanilla grandpa and aura consensus now. No migration is needed.
* moved `sc-consensus-aura` from input-output-hk/polkadot-sdk fork to this repository,
  to `sc-partner-chains-consensus-aura` and `sp-partner-chains-consensus-aura`.
  This change requires migration of the node, PartnerChainsProposerFactory has to be used.
  See `service.rs` in `partner-chains-node` crate for an example.
* renamed sidechain-main-cli and relevant naming to pc-contracts-cli
* ETCM-6092 - removed the `epoch-derivation` crate, moving epoch derivation logic to the `sidechain-domain`
crate. Removed `EpochConfig` wrapper type; code using it should be changed to use `MainchainEpochConfig`
type directly, `EpochConfig::read()` uses should be replaced by `MainchainEpochConfig::read_from_env()`.

## Fixed
* ETCM-8267 - fixed `partner-chains-cli` missing the native token configuration

## Added
* ETCM-7811 - native token movement observability components: `sp-native-token-management` and
`pallet-native-token-management` crates; data sources behind the `native-token` feature in
`main-chain-follower-api` and `db-sync-follower` crates.
* added helper functions to `SidechainParams` and all `MainChainScripts` types to read them from environment
* Extrinsic `set_main_chain_scripts` for migrating to new committee selection main chain scripts

# v1.0.0rc1

## Changed

* polkadot-sdk dependency updated to partnerchains-stable2407 (stable2407 is v1.15.0 in the previous scheme)
* changed the inner type of `McBlockHash` from Vec to an array
* ETCM-7985 - bumped `partner-chains-smart-contracts` version and updated the `partner-chains-cli`
to match. Now `partner-chains-cli` passes a network parameter to `sidechain-main-cli` where necessary.
* governance authority key hash is now calculated in `prepare-configuration` without using external `cardano-cli`

## Removed

* removed Relay docker build files
* removed usage of 'storage::getter' macros, following polkadot-sdk changes

# v1.0.0

## Changed
* ETCM-7498 - update to polkadot v1.12.0
* ETCM-6262 - changed the way benchmarks are used in runtime. It now contains 'weights' directory with benchmarks for pallets, block and extrinsics overheads, machine and storage. New scripts for generating them were added.
* ETCM-7506 - moved `generate-signatures` functionality to node command `registration-signatures`, output has changed, automation in this repository has been updated
* Extracted SidechainParams to own crate, encouraging users to create their own SidechainParams.
* ETCM-6826 - added runtime api for candidate verification. Candidate verification outside of runtime is done only through runtime api. This node requires runtime to have a spec_version >= 111.
* ETCM-7599 - create custom index from dbsync follower implementation, to avoid using custom db-sync image
* ETCM-7512 - added registration-status command to sidechains-substrate-node cli
* RocksDB is explicitly made a sc-client-db backend
* ETCM-7610 implemented 'establish bootnode' step from prepare-configuration wizard
* Rust toolchain and docker builder images are updated to 1.77.0
* ETCM-7611 implemented 'establish sidechain parameters' step from prepare-configuration wizard
* ETCM-7613 implemented 'create-chain-spec' wizard
* ETCM-7614 implemented 'setup-main-chain-state' wizard
* ETCM-7530 - Removed active flow from runtime and node. To migrate, remove the following dependencies from you node
and runtime crates: `sp-active-flow`, `pallet-active-flow`, `pallet-active-flow-rpc`, `sp-incoming-tx`,
`allet-incoming-tx` and `allet-incoming-tx-rpc`. This involves removing Active Flow Pallet and Incoming Transactions
Pallet from the runtime, along with their configuration, chain spec configuration and RPC modules in the node.
* Removed the `runtime-constants` crate. `SLOTS_PER_EPOCH` parameter now needs to be defined by the
users in their runtime crate.
* ETCM-7762 - update to polkadot v1.13.0 - requires adaption in node code, because GenericChainSpec has lost one generic parameter
* ETCM-7759 - decoupled all crates that use chain follower data sources from the concrete `DataSources` type.
Now all logic accepts any type that implements one of the traits: `XXXDataSource` or `HasXXXDataSource`.
* ETCM-7766 - updates to accommodate to new version of partner-chains-smart-contracts: containers and devnet settings and utilities
* BREAKING: ETCM-7818 read candidates related main chain configuration from ledger, not from environment. Migration for existing chains is to put proper configuration in the ledger and then export chain as a spec file.
* ETCM-7855 - removed all Active Flow-related components and features. This change should not affect
Partner Chains nodes that did not use Active Flow.
* Clean up docs to support independent release from different repo
* Modify publish-doc.yml to only publish rustdocs
* Removed the `manage-devenv` crate
* ETCM-7938 - update polkadot-sdk to v1.14.0
* Renamed `sidechain_domain::BlockHash` to `McBlockHash` for clarity
* BREAKING: ETCM-7950 - Move SLOTS_PER_EPOCH to const to storage value in order to enable its configuration without recompilation. It is NOT BREAKING change for the chains having SLOTS_PER_EPOCH equal to 60.
* `sidechain-runtime` and `sidechains-substrate-node` crates are updated to version 1.0.0
* renamed `sidechains-substrate-node` to `partner-chains-node`

## Fixed
* ETCM-7745 - do not overwrite legacy chain params in partner-chains-cli prepare-configuration wizard
* ETCM-7713 - partner-chains-cli prepare-configuration small improvements and fixes

## Added
* ETCM-7610 implemented 'establish main chain configuration' step of prepare-configuration wizard

# v0.2.1

## Changed
* ETCM-7422 - decoupled all pallet crates from their related inherent data crates. Moved all primitives
(types, error types, runtime APIs etc.) to separate crates in `primitives/` directory. Now for each
`pallet-*` crate there is a corresponding `sp-*` crate.
* ETCM-7423 - moved `McHashInherentDataProvider`, `IncomingTransactionsProvider`, `AriadneInherentDataProvider` and `CrossChainSignaturesInherentDataProvider`
creation logic out of the node crate, to their respective primitive crates.
Cleaned `inherent_data.rs` which now contains only wiring and necessary minor helper logic.
* ETCM-6709 - db-sync implementation of CandidatesDataSource has built-in caching, that is used only for stable main chain data. main-chain-follower-caching create is removed.
* Inherent data provider creation logic now uses error types instead of boxed errors where it's possible
* renamed the `block-rewards-primitives` crate to `sp-block-rewards` for consistency
* made `pallet-block-rewards` crate depend on `sp-block-rewards` and moved some types and traits there

## Fixed
* ETCM-7463 - fixed incoming transactions cache in db-sync-follower, it was not working at all

# v0.1.0

## Added
* ETCM-7080: add aura and grandpa public keys to registrations data returned from RPC method

## Changed
* IMPORTANT: partner-chains-smart-contracts revision has been updated in flake.nix.  Downstream projects, like Midnight, should keep using the previous value, to keep configuration utilities in sync with their testnets.
* BREAKING: ETCM-5905 - remove all storage maps from pallet-active-flow

* ETCM-7136 - replaced the local partner-chains-session pallet with one from the SDK
* ETCM-7144 - fixes for the mainchain-follower-cli, required changes in mainchain-follower-api
* ETCM-7293 - modularized the MC follower API, db-sync implementation and mock crates,
hiding each data source type behind a feature
* ETCM-6955 - move incoming transactions cache to db-sync-follower and use updated lower bound when after_tx param is not defined
* BREAKING: ETCM-7330 - pallet-sidechain works with generic SidechainParameters. Change is breaking because storages are now generic over SidechainParameters.
  sidechain_getParams returns JSON representation of generic parameters instead of concrete GetParamsResponse.
  sidechain_getEpochSignatures field 'params' has changed - Relay 2.6.x is not compatible with this change, use Relay 2.7.x.
* ETCM-7108 - unknown or 0 stake delegation makes registration invalid. It should not change committee selection results.
* Refactoring: moved `sidechain_getAridaneParameters` and `sidechain_getRegistrations` to `pallet-session-validator-management-rpc`.

## Fixed
* ETCM-7080 - use the same candidate validation as in runtime for `sidechain_getRegistrations` and `sidechain_getAriadneParameters` rpc methods.

# v0.0.26

## Added

* Added an optional `slot` parameter to `getEpochPhase` json RPC method
* BREAKING: ETCM-6767 - store and verify mainchain block hash in header
* ETCM-6766 - main chain follower implementation for getting blocks for main chain state reference
* ETCM-6765 - add BLOCK_STABILITY_MARGIN to the configuration
* ETCM-6855 - incoming transactions with invalid recipient are filtered out when reading inherents, RPC method `sidechain_getIncomingTransactions` doesn't fail when there are transactions with invalid recipients.
* ETCM-6777 - verify that mainchain state reference slot is high enough to calculate next committee
* ETCM-6954 - update to partnerchains-polkadot-v1.9.0 (public fork), update most of the dependencies

## Changed
* BREAKING: ETCM-5898 - remove storage maps from pallet-session-validator-management and simplify the committee rotation logic
* ETCM-6877 - improved the performance of getting the latest on chain committee hash, requires update in deployment configuration.
* ETCM-6822 - update partner-chains-smart-contracts revision to: 76f57380b6d85f2c1a1f212591a99ebd0db96213.
* ETCM-6816, ETCM-6813 - removed dependency on `sidechain-inherents`, `mock-types` and `sidechain-domain` from `pallet-session-validator-management` crate
* ETCM-6813 - moved authority selection code from `sidechain-inherents` to a new `authority-selection-inherents` crate
* ETCM-6813 - moved code shared between `authority-selection-inherents` and `pallet-session-validator-management`
to a new crate `sp-session-validator-management`
* BREAKING: ETCM-6777 - all ariadne main chain data used for selecting the committees for partner chain epochs which take place during main chain epoch N is now sourced from last slot of main chain epoch N-2
* changed logic in `authority-selection-inherents` crate to operate on Sr25519 and Ed25519 public key
types instead of raw byte arrays
* BREAKING: ETCM-6776 - use MC hash in the incoming transactions calculation
* ETCM-7072 - move `ValidatorManagementSessionManager` to a dedicated crate

## Fixed
* ETCM-6854 - make endpoints_spec.json file consistent with the actual implementation in session-validator-management
* ETCM-7051 - sidechain_getEpochSignatures nextCommitteePubKeys were returning next committee of the current epoch
* ETCM-7081 - fixed 'check_inherent' when committee cannot be selected from inherent data
* ETCM-7115 - fixed `sidechain_getEpochCommittee` RPC method returning wrong committee for next epoch from now
* ETCM-7143 - fixed invalid block queries for the stable block at in dbsync-mainchain-follower

# v0.0.25

## Added

* ETCM-6517 - Added a main chain hash inherent provider and digest item under ID `mcsh`.

## Changed

* ETCM-6858 - modified queries for getting registered candidates, permissioned-candidates, and d-parameter, vastly improving their performance.
* ETCM-6655 - refactor/cleanup of main chain follower queries related to registrations.
* ETCM-6517 - Updated Polkadot SDK dependency to a version introducing inherent digests.
* ETCM-6818 - remove pallet session validator management dependency to pallet sidechains session
* ETCM-6814 - decouple session-validator-management pallet from sidechain pallet.
* ETCM-6600 - updated the Polkadot SDK version to 1.7

# v0.0.24

## Added

* ETCM-6777 - chain initialization: insert initial committee into the storage
* ETCM-6608 - chain initialization: dumb search algorithm to find the earliest committee, plus `MINIMUM_MC_EPOCH` config setting for optimisation
* ETCM-6629 - `sidechain_getEpochPhase` json RPC method in `active-flow-pallet`
* ETCM-6648 - added `limit` parameter to `sidechain_getSignaturesToUpload` json RPC method

## Changed

* ETCM-6517 - `sidechain_getRegistrations` changed to return only active and invalid registrations in `mainchainEpoch` for `mainchainPublicKey`
* ETCM-6517 - `sidechain_getAriadneParameters` extended with `candidateRegistrations` response field
* ETCM-5756 (update) - increased granularity of errors when retrieving the parameters
* ETCM-6629 - moved `sidechain_getStatus` json RPC method to `sidechain-pallet`
* ETCM-6728 - changed `sidechain_getStatus` json RPC method to calculate `epoch`
and `nextEpochTimestamp` using current time

## Removed

* ETCM-6629 - removed `epoch_phase` field from `sidechain_getStatus` json RPC method

## Fixed

* ETCM-6616 inherent data errors are handled properly, so meaningful messages are displayed

# v0.0.23

## Added

*  ETCM-6075 - added prometheus metrics `execution_time`, `call_count` for each method in main chain follower services

## Changed

* ETCM-6420 - update the Polkadot SDK dependency to 1.6.0
* ETCM-5905 - use block search algorithm in `sidechain_getOutgoingTxMerkleProof` - enables replacing StorageMaps with StorageValue and save space.
* ETCM-5905 - update sidechain_getOutgoingTxMerkleProof to use block search

## Fixed

* ETCM-6536 - fixed rejection of blocks without incoming transactions ('unlock' call), when verifier can already observe some new transactions on the main chain.
Fixed accepting a block with 'unlock' call when verifier does not see any transactions.

# v0.0.22

## Fixed

* ETCM-6367 - fixed `sidechain_getEpochsToUpload`, so it returns the first epoch of sidechain by @LGLO in https://github.com/input-output-hk/partner-chains/pull/521
* ETCM-6369 - fixed invalid incoming transaction inherent caused by negative amount (bug in db-sync main chain follower) by @LGLO in https://github.com/input-output-hk/partner-chains/pull/527
* ETCM-5479 - fixed merkle proof endpoint with proper distributed set utxo by @AmbientTea in https://github.com/input-output-hk/partner-chains/pull/532
* ETCM-6441 - fixed a SQL query, so it can use index on multi_asset table when looking for d-parameter and permissioned candidates by @LGLO in https://github.com/input-output-hk/partner-chains/pull/537

## Added

* ETCM-6080 - mock payouts to block beneficiaries by @AmbientTea in https://github.com/input-output-hk/partner-chains/pull/500

## Changed

* ETCM-5898 - Updated `sidechain_getEpochCommittee` to search for appropriate block, will allow to not store map of all epochs in the future by @aang114 in https://github.com/input-output-hk/partner-chains/pull/535
* ETCM-5756 - Unified `get_permissioned_candidates_for_epoch` and `get_d_parameter_for_epoch` into a single function: `get_ariadne_parameters`
* ETCM-6516 - Updated slot assignment algorithm with the most recent specification https://github.com/input-output-hk/partner-chains/pull/563
