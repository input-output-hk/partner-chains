# Changelog

This changelog is based on [Keep A Changelog](https://keepachangelog.com/en/1.1.0).

# Unreleased

## Changed

## Removed

## Fixed

## Added

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

## Removed

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

## Removed

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

## Removed

## Fixed
* ETCM-7463 - fixed incoming transactions cache in db-sync-follower, it was not working at all

## Added

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

## Removed

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

## Removed

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

## Removed

## Fixed

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
