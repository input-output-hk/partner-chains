# Native Token Management - Migration Guide

## About

This document describes how to add the Native Token Management features to an already running
partner chain and how to remove it.

## Context

The Native Token Management feature in the Partner Chain Toolkit consists of the pallet,
the primitives crate, and the supporting DB-Sync data source. It complements the main chain component by providing its observability.

Care must be taken when adding this feature to a running chain, because the data source and inherent
data provider require runtime data for operation.

## Obtaining the main chain scripts

The native token management observability is configured using the following parameters:
* `illiquid supply validator address` - this is the address to which the native tokens are sent on the 
main chain to lock them so that they can be unlocked on the partner chain. This address is derived
from the sidechain params and is present in the output of the `addresses` command of the
`partner-chains-smart-contracts` CLI under `addresses/IlliquidCirculationSupplyValidator`.
When using the `parner-chains-cli` wizard, it will be automatically set up in the `prepare-configuration` step.
* native token minting `policy ID` and `asset name` - those should come from the Cardano Native Asset
used to represent the partner chain's token on the main chain. Because each native asset used can have
different logic in its minting policy, development and creation of the asset is left for each
partner chain builder.
When using the `parner-chains-cli` wizard, these can be set up in the `prepare-configuration` step.

## Migration Steps - adding the feature

1. Add the pallet into the runtime. This requires implementing the trait `TokenTransferHandler`, which
is left for the developers of each particular partner chain to implement according to their needs and
ledger structure. Consult the implementation in `runtime/src/lib.rs` for an example with a mocked handler.
2. Bump the runtime spec version and note its new value, we will refer to it as `start_version`.
3. Wire the data source into the node.
Consult the reference implementation in `node/src/main_chain_follower.rs` for an example.
4. Add the inherent data provider to your inherent data provider creation logic.
Consult the implementation in `node/src/inherent_data.rs` for an example.
Because the inherent data provider should only run after the pallet is added and fully configured
in the runtime, `new_for_runtime_version` factory function should be used, which accepts an additional predicate as argument, making it possible to specify the correct runtime version range:
in this case `|version| version.spec_version >= start_version` should be used.
5. Upgrade the nodes running the chain to the new node version containing the IDP added in previous steps.
6. Perform a runtime upgrade (using `sudo` or other governance feature) to the spec version `start_version` containing the pallet.
7. Invoke the `set_main_chain_scripts` extrinsic on the newly added pallet, using the governance mechanism,
to set the native token `policy ID` and `asset name`, and the `illiquid supply validator address`. After
this step all information necessary is present, and the native token data provider will start producing
the inherent data based on observed native token transfers, triggering the pallet's inherent when necessary. 


## Migration Steps - removing the feature

1. Decide on a runtime version `stop_version` from which the native token observation should cease.
2. Update the runtime version range predicate for `new_for_runtime_version` to use this runtime version as a limit.
Assuming the "adding the feature" scenario was implemented earlier, the version bound will look like
this: `|version| version.spec_version >= start_version && version.spec_version < stop_version`.
3. Upgrade the nodes in the network to this new version.
4. Remove the pallet completely from the runtime. The `TokenTransferHandler` can be removed as well.
Bump the runtime spec version to `stop_version` with these changes.
5. Perform runtime upgrade.
After this step _no further native token movement will be observed, even if performed on the main chain_.

---
**NOTE**

To support syncing and validating historical blocks, the data source and inherent data provider
*must* not be removed from the node.

---
