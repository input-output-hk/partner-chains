# Partner Chains Toolkit Documentation

## Table Of Contents

* [Table Of Contents](#table-of-contents)
  * [Basics](#basics)
    * [What is a Partner Chain](#what-is-a-partner-chain)
    * [Shared security with Cardano](#shared-security-with-cardano)
    * [Mixed validator committee](#mixed-validator-committee)
    * [Registered and Permissioned Validators](#registered-and-permissioned-validators)
      * [Registered Validator](#registered-validator)
      * [Permissioned Validator](#permissioned-validator)
  * [System Overview](#system-overview)
    * [db\-sync](#db-sync)
    * [ogmios](#ogmios)
    * [cardano node](#cardano-node)
  * [Features](#features)
    * [Features Overview](#feature-overview)
    * [Block Participation Rewards](#block-participation-rewards)
    * [Partner Chains Governance](#partner-chains-governance)
    * [Native Token Reserve Management](#native-token-reserve-management)
  * [Rust Docs](#rust-docs)
  * [Upgrade &amp; Migration Guides](#upgrade--migration-guides)
      * [1\.3\.0 to 1\.3\.1](#130-to-131)
      * [1\.3\.1 to 1\.4\.0](#131-to-140)
      * [1\.3\.1 to 1\.4\.0](#131-to-140-1)

### Basics

The Partner Chain Toolkit provides tools and features to build and maintain partner chains that are
secured by the Cardano main chain. This documentation provides general information and
usage instructions for getting started.

#### What is a Partner Chain
A partner chain is a specialized blockchain that runs in parallel to Cardano. Partner chains
leverage Cardano's advanced and established security while maintaining their own specialized focus
and functionality.

#### Shared security with Cardano
Cardano has a robust set of Stake Pool Operators (SPOs) with a proven history of providing security to the Cardano network. The Partner Chains Toolkit allows those SPOs to bring that same security to new partner chains, right out of the box.

The partner chain maintains awareness of Cardano's state through a chain indexer, which constantly updates the partner chain with relevant information from Cardano.

Network parameters stored securely on Cardano can be viewed from the partner chain, helping the partner chain defend against possible parameter attacks. Cardano also provides a native token reserve management system that the partner chain can use to store treasury tokens.

Cardano has a large, welcoming ecosystem of developers, applications, and users that may provide liquidity to a partner chain. Partner chains could decide to make use of Cardano’s supporting infrastructure such as chain explorers and wallet integrations.

#### Mixed validator committee
**Ariadne**

One of the distinguishing features of partner chains is the ability to establish any consensus model, including those with permissioned validators. This algorithm, _Ariadne_, is a novel selection algorithm that reads committee candidates and parameters from Cardano. Its output is designed to be fed to whatever consensus algorithm is in use.

The Ariadne algorithm combines both registered Cardano SPOs and permissioned validators to form the block production committee for a partner chain. Each committe selection persists for one epoch. The algorithm selects a new combination of validators for the next epoch.

**The D parameter**

The D parameter sets the percentage of blocks to be produced by trusted members of each block production committee. This feature safeguards a protocol in its early stages from potential malfeasance by the committee, while also allowing for the gradual transition towards full decentralization.

Ariadne selects a mix of permissioned and registered validators for each epoch, creating a committee that balances trust and decentralization. This selection process feeds into the Aura consensus protocol used by the partner chain, resulting in a robust and flexible block validation method.

The builder of the partner chain, acting as the governance authority, nominates the permissioned, trusted committee members as a whitelist of public keys. In partner chains, a new block production committee is selected for each epoch by a pseudo-random process. The governance authority can set the D parameter to control the average ratio of permissioned members to registered members in the block production committee.

Either number in the ratio can be zero, allowing the blockchain to begin with a fully permissioned committee and migrate to a fully decentralized committee with only registered members.

Blocks are allocated to validators in proportion to the distribution of their stake pool delegations.

#### Registered and Permissioned Validators
Validators secure the integrity of the partner chain and can be differentiated in two groups:
- **registered**, and
- **permissioned**

Both types of validators are described below

##### Registered Validator
A registered validator is a Cardano SPO who has chosen to become a partner chain validator by meeting certain requirements. They must post a signed registration message specific to the partner chain. They will then contribute to block production on that partner chain. This is a permissionless role that requires Cardano SPO keys.

##### Permissioned Validator
A permissioned validator is a trusted node whitelisted by the governance authority to produce blocks on the partner chain. The whitelist of permissioned nodes is created by the chain builder acting as the governance authority. This node must run a partner chain node with Cardano node and DB Sync. It may be a Cardano SPO but that is not required.

### System Overview
The diagram below provides an simple overview of the pc toolkit setup:

<p align="center">
  <img src="./diagrams/pc-overview.drawio.svg" alt="" />
</p>

The toolkit covers components across three different categories which have been color-coded in the
diagram above:

1. <span style="background-color: #B9E0A5;">Substrate Node</span>: This includes runtime modules
   and the Ariadne consensus
1. <span style="background-color: #A9C4EB;">offchain/cli</span>: The offchain components of the partner
   chains toolkit deploy  and call smart contracts.
1. <span style="background-color: #FFE599;">smart contracts</span>: The smart contracts that are
   deployed and called by the offchain components.

**_Note_**: _It is worth mentioning that the diagram above outlines the full setup which a chain builder
would need to run in order to operate a chain. Validators won't need to use offchain components and
thus won't have to run ogmios_.


#### db-sync
[db-sync](https://github.com/IntersectMBO/cardano-db-sync) is a chain-indexer which follows the
cardano chain and stores ledger state changes to a connected PostgreSQL database. In order to
observe and respond to events on the main-chain the substrate node components query the persisted
ledger state in the database to look up specific blocks, transactions or addresses.

#### ogmios
The offchain code connects to an [ogmios](https://github.com/CardanoSolutions/ogmios) instance,
which is a lightweight bridge-interface providing a http/websocket api for communicating with the local cardano
node. This bridge is used by the offchain components of the toolkit to install and interact with the
necessary smart contracts.

#### cardano node
The [cardano-node](https://github.com/IntersectMBO/cardano-node) instance is shown on the right with several deployed smart contracts. The offchain
components of the partner chains toolkit deploy  and call smart contracts but otherwise the toolkit
will only ever observe the ledger state, not change it.


### Features

#### Features Overview

The diagram below gives an hierarchical overview of the different features provided by this toolkit and their
respective dependencies (where `a -> b`means that `b` depends on functionality provided by `a`):

<p align="center">
  <img src="./diagrams/features.svg" alt="" />
</p>

- **primitives and utils**: Utility libraries and custom Substrate primitives used by all other
features.
- **core**: Establishes a chain as a Partner Chain by tying its identity to a `genesis utxo` on Cardano. Provides the mechanism for the Partner Chain's blocks to reference stable Cardano blocks.
- **governed map**: Governance controlled key-value store on the Cardano main chain.
- **native token management**: Provides governance controlled tokens and token reserve management.
- **address association**: Provides a mechanism for users to establish a mapping between their identities on Cardano and the Partner Chain
- **committee selection**: Provides the cardano-based committee selection using the ariadne algorithm.
- **cardano based block production rewards**: Calculation of rewards for delegation and block
production on cardano.

More detailed documentation for the different features is provided in the sections below.

#### Block Participation Rewards
Please refer to [block-participation-rewards.md](./developer-guides/block-participation-rewards.md)
to learn about mechanisms to build and configure rewards for block producers and their delegators.

#### Partner Chains Governance
Please refer to [governance.md](./user-guides/governance/governance.md) for insights into how to use
the governance system to control the D-Parameter, the permissioned candidate list and the rewards
mechanism.

#### Native Token Reserve Management
Please refer to
[native-token-reserve-management.md](./developer-guides/native-token-reserve-management.md) for
details on how to setting up and maintaining a native token reserve on Cardano to  be used with a
partner chain.

### Rust Docs

Rust Docs for all crates provided by the toolkit are available to browse online:
[https://input-output-hk.github.io/partner-chains/](https://input-output-hk.github.io/partner-chains/)

### Upgrade & Migration Guides
The following migration guides outline how to upgrade a running partner chain from one version to
another in a safe and non-disruptive ways:

##### 1.3.0 to 1.3.1
[Upgrading from 1.3.0 to 1.3.1](./developer-guides/sdk-update-v1.3.0-to-v1.3.1.md)
##### 1.3.1 to 1.4.0
[Upgrading from 1.3.1 to 1.4.0](./developer-guides/sdk-update-v1.3.1-to-v1.4.0.md)
##### 1.3.1 to 1.4.0
[Upgrading from 1.3.1 to 1.4.0](./developer-guides/sdk-update-v1.3.1-to-v1.4.0.md)

[Upgrading a running partner chain from 1.3.1 to 1.4.0](./developer-guides/migration-guide-1.4.0.md)
