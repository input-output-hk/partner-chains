# Versioned Plutus Data - Migration Guide

## About

This document describes steps needed to migrate a running chain to use the new smart contracts
that introduce versioned Plutus data.

## Context

To make it easier for Partner Chains to keep up with the evolution of the Partner Chains SDK
while avoiding chain resets and other disruptions, the new Partner Chains Smart Contracts version
introduces a new layout of on-chain data in the form of *generic versioned Plutus data*. This
means that every datum attached to UTXOs produced by the Partner Chains tooling and observed by
the node, now follows a common structure with:
- **smart contract-specific data section** with a schema fixed by the smart contract version
- **generic data section**, whose schema can freely evolve without smart contract changes
- **version number**

## Migration steps - Ariadne

At the moment, the only feature actively supported by the Partner Chains SDK which uses Plutus
data observability is the Ariadne committee selection mechanism, which observes the following
data:
- D-Parameter
- Permissioned candidates list
- Cardano SPO registrations

The outline of the migration can be summarized as:
1. New smart contract release
1. Node data source upgrade
1. Transitionary period
1. Observed addresses change

### Smart contract redeployment

Introducing versioned Plutus data is a breaking change in the smart contracts, which means
new versions of the smart contracts will need to be used. The governance authority should use the
new version of Partner Chains Smart Contracts to set up a new D-Parameter and permissioned 
candidate list on the main chain. The new version of the tooling should also be made available
to the Cardano SPOs running the nodes in the network.

### Node data source upgrade

For the nodes in the network to be able to observe the new Plutus data format, the node needs to
be upgraded to the new version of Partner Chains SDK-provided data sources. This only involves
updating the on Partner Chains SDK's Substrate dependencies to a newer version, since no API
changes were made. A version of the node with upgraded data source needs to be made available to
all participants of the network.

### Transitionary period

At this point a transitionary period should be established when the chain would still
operate using the old smart contract data. During this period:

- all SPOs wishing to continue participating in the chain will need to register themselves
again using the new smart contracts.
- nodes running in the network will need to be updated to the version released in the previous step.
At the time of upgrade, no changes to observed addresses should be made, as the new data sources 
are capable of observing both versioned datums and the legacy schema for backwards compatibility.

### Observed addresses change

After the transitionary period has ended and enough of the nodes running the chain have been upgraded
and SPOs re-registered themselves, the addresses observed for committee selection would need to be
changed using the `set_main_chain_scripts` extrinsic and the `sudo` pallet or other on-chain
governance mechanism.

After this step the chain will start using the new smart contract data containing versioned datums and
further updates to the datum schemas will no longer require smart contract changes.
