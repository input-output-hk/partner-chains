# Native Token Management - Migration Guide

## About

This document describes how to add the native token management capabilities to an already running
Partner Chain.

## Context

The native token management system in the Partner Chain SDK Substrate stack consists of the pallet,
the primitives crate containing the inherent data provider and the supporting DB-Sync data source.
It complements the main chain part by providing its observability.

Care must be taken when adding this feature to a running chain, since the data source and inherent
data provider require runtime data for operation.

## Obtaining the main chain scripts

The native token management observability is configured using the following parameters:
* `illiquid supply validator address` - this is the address to which the native tokens are sent on the 
main chain to lock them so that they can be unlocked on the Partner Chain. This address is derived
from the sidechain params and is present in the output of the `addresses` command of the
`partner-chains-smart-contracts` CLI under `addresses/IlliquidCirculationSupplyValidator`.
* native token minting `policy ID` and `asset name` - those should come from the native Cardano asset
used to represent the Partner Chain's token on main chain. Because each native asset used can have
different logic in its minting policy, development and creation of the asset is left for each
Partner Chain developer team.

## Migration scenario - adding the feature

#### ATTENTION: If migration described below is performed, only native token transfers that have taken place _after the MC hash referenced by the block after runtime upgrade in step 7_ will be observed. If there have been earlier transfers to the illiquid supply that your Partner Chain needs to account for, use a different migration strategy or wait for a newer version of the native token management pallet!

1. Add the pallet into the runtime. This requires implementing the trait `TokenTransferHandler`, which
is left for the developers of each particular Partner Chain to implement according to their needs and
ledger structure. Consult the implementation in `runtime/src/lib.rs` for an example with a mocked handler.
2. Perform a runtime upgrade using the usual mechanism (`sudo` or other governance feature). This requires
a spec version number bump.
3. Invoke the `set_main_chain_scripts` extrinsic on the newly added pallet, using the governance mechanism,
to set the native token `policy ID` and `asset name`, and the `illiquid supply validator address`.
4. Pick a runtime spec version `nt_launch_v` higher than the one after step 3. This runtime version will
be the one after which native token observation will be processed.
5. Wire the data source into the node.
Consult the reference implementation in `node/src/main_chain_follower.rs` for an example.
6. Add the inherent data provider into the inherent data provider creation logic.
Consult the implementation in `node/src/inherent_data.rs` for an example.
Because the inherent data provider should run only once the pallet and the runtime scripts are available
in the runtime, `new_for_runtime_version` factory function should be used to specify the correct runtime
version range, in this case `|version| version.spec_version >= nt_launch_v`.
7. Perform another runtime upgrade, bumping the runtime spec version to or above `nt_launch_v`.
Once the upgrade is done, the native token data provider will start producing the inherent data based
on observed native token transfer, triggering the pallet's inherent. 


## Migration scenario - removing the feature

1. Decide on a runtime version `nt_stop_v` from which the native token observation will cease.
2. Update the runtime version range for `new_for_runtime_version` to use this runtime version as a limit.
Assuming the "adding the feature" scenario was implemented earlier, the version bound will look like
this: `|version| version.spec_version >= nt_launc_v && version.spec_version <= nt_stop_v`.
3. Upgrade the nodes in the network to this new version. After this step
_no further native token movement will be observed, even if performed on the main chain_.
4. Remove the pallet completely from the runtime. The `TokenTransferHandler` can be removed as well.
Bump the runtime spec version to `nt_stop_v` with these changes.
5. Perform runtime upgrade.
