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
    * [General Overview](#general-overview)
    * [db\-sync](#db-sync)
    * [ogmios](#ogmios)
    * [cardano node](#cardano-node)
    * [System Design](#system-design)
  * [Running Partner Chain Dependencies](#running-a-partner-chain-dependencies)
    * [Running cardano-node](#running-cardano-node)
    * [Running db-sync](#running-db-sync)
    * [Running PostgreSQL](#running-postgresql)
    * [Running ogmios](#running-ogmios)
  * [Wizards](#wizards)
    * [generate-keys](#generate-keys)
    * [prepare-configuration](#prepare-configuration)
    * [create-chain-spec](#create-chain-spec)
    * [setup-main-chain-state](#setup-main-chain-state)
    * [start-node](#start-node)
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
#### General Overview
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

#### System Design

<p align="center">
  <img src="./diagrams/feature-design.svg" alt="" />
</p>

The diagram above illustrates a typical structure of a feature provided by this toolkit.

**Inside the Runtime**, the ledger rules of any feature are implemented by a _FRAME Pallet_. This pallet
defines what data is stored on-chain and what transactions (*extrinsics* in Substrate terminology) are
available for the Partner Chain's users to submit. Most pallets also define their own internal transactions
– called *inherent extrinsics* in Substrate – that are run by the system itself for operational reasons.
These inherents are often run to handle some data observed on the Cardano main chain in the Partner
Chain's ledger. The pallet is also responsible for storing the feature's configuration to make it subject
to the consensus mechanism.

**The Node components** of a feature are mostly responsible for mediating between the runtime pallet and
the outside world. These include the _inherent data providers_ that make data from the outside world
available to the runtime for processing, and _RPC endpoints_ that runtime data to the users over Json RPC.
For querying runtime data from the pallets, they use Substrate's _Runtime APIs_ exposed by the runtime.

**Inherent Data Providers** are a particularly important type of node components, as they are responsible
for supplying trusted, system-level data during block production. Different features require different
external data to operate. The category of inherent data characteristic of Partner Chains Toolkit is
_Cardano observability data_ used by features that provide the ability to source security and operational
data from the Cardano main chain. For the sake of modularity and indexer-independence, each feature separately
defines its data needs in the form of a _Data Source API_. This APIs serve as contracts for various concrete
_Data Source Implementations_, which need to be aware of low-level concerns like concrete indexer APIs and
physical layout of on-chain Plutus data.

Both runtime and node components make use of foundational types, traits and utilities that are necessary for
various parts of a feature to interoperate. These are implemented in the feature's _Primitives_ crate which
is depended on by all other crates that implement that feature.

For features that require observable data on the Cardano main chain, an important component are their
**Plutus Scripts** which are Plutus **smart contract** code that is deployed to Cardano, along with their
_Offchain_ code, which provides logic for building and executing Cardano transactions.

Finally, many features expose **Cli Commands** that support their operation. These include commands to interact
with the Cardano main chain using the offchain code, create various signatures, and query the Partner Chain's
state and configuraiont.

### Running Partner Chain Dependencies
In order to run a partner chain, several services must be running on the same network as the node
instance. All of these services are external projects and provide their own documentation. This
documentaion will mostly refer to existing documentation but point out any specifics relating to
using these services in the context of a partner chain setup.

#### Running cardano-node
[cardano-node](https://github.com/IntersectMBO/cardano-node) is the core service for connecting to
the Cardano blockchain and is mandatory for running a partner chain. Please refer to the [project
website](https://github.com/IntersectMBO/cardano-node) for detailed instructions on configuring and
running cardano-node.

:warning: Please note that your cardano-node instance needs to be fully synchronized before you can
start to create a partner chain setup. The synchronization time depends on the network conditions and
hardware characteristics but below are some approximations:

| network  | approximated sync time |
| ------------- | ------------- |
| preview       | hours  |
| pre-prod      | up to a day|
| mainnet       | ~2 days|

#### Running db-sync
In order to observe the state of the Cardano ledger, the partner chain relies on
[db-sync](https://github.com/IntersectMBO/cardano-db-sync) as chain-indexer. The default
configuration is sufficient. Make sure to refer to the project
[documentation](https://github.com/IntersectMBO/cardano-db-sync/blob/master/doc/Readme.md) for
further details.

:warning: Please note that db-sync needs to be **fully synchronized**. Attempting to run a partner chain node
with a db-sync instance that lags behind will result in consensus errors. The synchronization time
depends on the network conditions and hardware characteristics but below are some approximations:


| network  | approximated sync time |
| ------------- | ------------- |
| preview       | hours  |
| pre-prod      | up to a day|
| mainnet       | ~2 days|


#### Running PostgreSQL

A [PostgreSQL](https://www.postgresql.org/) database is a runtime requirement via db-sync as the
indexer persists ledger state and events in the database. The partner chain node also needs to
access the database directly.

:warning: Make sure to create a database called `cexplorer` and make it accessible to the user which
will execute your partner chain node executable.


#### Running ogmios
[ogmios](https://github.com/CardanoSolutions/ogmios) is a lightweight bridge-interface providing a
http/websocket interface for communicating with a local cardano node. Unlike _db-sync_, which is
always mandatory, ogmios is only required when interacting with the smart contracts as chain
builder, or when registering as SPO.

Please refer to the [project
documentation](https://ogmios.dev/getting-started/building/#-documentation) for details on how to
run and configure ogmios.

### Wizards
The Partner Chain toolkit provides several wizards that serve as convenience layer to carry out
configuration or bootstrapping actions. The tasks performed by the wizards can also be carried out
by interacting with different commands directly. If you prefer full control over convenience of use,
you don't _have_ to use the wizards.

All wizards are available as sub-commands to the `wizards` command. Passing `--help` will list all
available wizards:
```shell
$ pc-node wizards --help
```

#### generate-keys
The `generate-keys` wizard creates all necessary keys and saves them to the node's default keystore
location. The following three keys will be created:

- cross-chain key (ECDSA)
- grandpa key (ED25519)
- aura key (SR25519)

Additonally, the wizard will generate a network key if it doesn't exist already.

**Prerequisites**

none

**Output**

- `pc-resources-config.json`
- `partner-chains-public-keys.json`

**Running the wizard**

```shell
$ pc-node wizards generate-keys
```
A successful run will conclude with output similar to the one below (**note**: output is abbreviated):
```
 🔑 The following public keys were generated and saved to the partner-chains-public-keys.json file:
 {
   "sidechain_pub_key": "0x02aec45eb3b0ee511be154e0daab9d3d77e9f5c2a1cbde075244727655bad12592",
   "aura_pub_key": "0xc074d26b17c018fba23c14b305871dc0126ac868c10a0e60bdeb0a353bd5e20f",
   "grandpa_pub_key": "0x6bdd6017b99b30885b544f51712ecfc0adc61f3e1835201a1aebcce1eb9dcee0"
 }
 You may share them with your chain governance authority
 if you wish to be included as a permissioned candidate.

 ⚙️ Generating network key
 running external command: mkdir -p ./data/network
 running external command: /tmp/partner-chain
 command output: 12D3KooWK9wfu9pmbUaCoTuU6eP4X3VmWWPGDdRVXK7T8KiCrTRm
```

</details>

#### prepare-configuration
The `prepare-configuration` wizard will guide you through the configuration needed to create a
governance authority. The main output of this wizard is the `pc-config.json` file.

**Prerequisites**

- The `ogmios` service must be up and running
- The wizard will ask for the payment signing key file `payment.skey`.
- The payment address must be funded

**Output**

- `pc-chain-config.json`
- `pc-resources-config.json`

Please refer to the `cardano-cli`
[documentation](https://developers.cardano.org/docs/get-started/cardano-cli/get-started/#generate-a-payment-key-pair-and-an-address)
for details on how to create keys.

**Running the wizard**

```shell
$ pc-node wizards prepare-configuration
```
A successful run will conclude with output similiar to the following
```
Governance initialized successfully for UTXO: f1885ad969f4e8572c34d281fe4f7e4f7c4e68183a9ea2c17961ef848da
3ea41#0
Cardano addresses have been set up:
- Committee Candidates Address: addr_test1wzy80cnj69fzm34pz6v9kzq3z3y8vwyy44t57ex3lu0w22gmqu0h4
- D Parameter Policy ID: fd08f51e106671e9129a4bb698066f018b393d7385986e103bce1fec
- Permissioned Candidates Policy ID: da8248dd4dc957e38b6f9f9a611c95a3bc46125df1bc801efdfe606f
- Illiquid Supply Address: addr_test1wzyxafwqnez8tlw59jg6m0q33gj3ker078pghvqv7v5yfgcm9qre4
Partner Chains can store their initial token supply on Cardano as Cardano native tokens.
Creation of the native token is not supported by this wizard and must be performed manually before this s
tep.
> Do you want to configure a native token for you Partner Chain? No
Chain configuration (pc-chain-config.json) is now ready for distribution to network participants.

If you intend to run a chain with permissioned candidates, you must manually set their keys in the pc-cha
in-config.json file before proceeding. Here's an example of how to add permissioned candidates:

{
  ...
  "initial_permissioned_candidates": [
    {
      "aura_pub_key": "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde49a5684e7a56da27d",
      "grandpa_pub_key": "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200f498922423d4334014fa6b0ee",
      "sidechain_pub_key": "0x020a1091341fe5664bfa1782d5e0477968906ac916b04cb365ec3153755684d9a1"
    },
    {
      "aura_pub_key": "0x8eaf04151687736326c9fea17e25fc5287613698c912909cb226aa4794f26a48",
      "grandpa_pub_key": "0xd17c2d7823ebf260fd138f2d7e27d114cb145d968b5ff5006125f2414fadae69",
      "sidechain_pub_key": "0x0390084fdbf27d2b79d26a4f13f0cdd982cb755a661969143c37cbc49ef5b91f27"
    }
  ]
}

After setting up the permissioned candidates, execute the 'create-chain-spec' command to generate the fin
al chain specification.
🚀 All done!
```

#### create-chain-spec

The `create-chain-spec` wizard creates a chain specification based on an existing
`pc-chain-config.json` file (which can be generated using the `prepare-configuration` wizard).

**Prerequisites**

- `pc-chain-config.json`

**Output**

- `chain-spec.json`

**Running the wizard**

```shell
$ pc-node wizards create-chain-spec
```

A successful run will conclude with output similar to the following
```
This wizard will create a chain spec JSON file according to the provided configuration, using WASM runtime code from the compiled node binary.
Chain parameters:
- Genesis UTXO: f1885ad969f4e8572c34d281fe4f7e4f7c4e68183a9ea2c17961ef848da3ea41#0
SessionValidatorManagement Main Chain Configuration:
- committee_candidate_address: addr_test1wzy80cnj69fzm34pz6v9kzq3z3y8vwyy44t57ex3lu0w22gmqu0h4
- d_parameter_policy_id: fd08f51e106671e9129a4bb698066f018b393d7385986e103bce1fec
- permissioned_candidates_policy_id: da8248dd4dc957e38b6f9f9a611c95a3bc46125df1bc801efdfe606f
Native Token Management Configuration (unused if empty):
- asset name: 0x
- asset policy ID: 0x00000000000000000000000000000000000000000000000000000000
- illiquid supply address: addr_test1wzyxafwqnez8tlw59jg6m0q33gj3ker078pghvqv7v5yfgcm9qre4
Initial permissioned candidates:
- Partner Chains Key: 0x03381169b112b614f8c5df1fb70d483d91d6b3b8d23af3b5e48eb0c0e7d090c57e, AURA: 0x5ab1f762b4f068d18000244ca2a1defa1ab7cfe7391521203c84323fbd1bfb73, GRANDPA: 0xaf350a984cc5ede7edec4e798edab89e0935b6250d447e212a29b94cf56114a1
> Do you want to continue? Yes
running external command: /tmp/pc-node build-spec --disable-default-bootnode > chain-spec.json
command output: 2025-05-15 15:47:58 Building chain spec
chain-spec.json file has been created.
If you are the governance authority, you can distribute it to the validators.
Run 'setup-main-chain-state' command to set D-parameter and permissioned candidates on Cardano.
```

The chain specification file created by this wizard is ready to be distributed to block production
committee candidates.

#### setup-main-chain-state

The `setup-main-chain-state` wizard configures the D-parameter and permissioned candidates list on
the main chain.

**Prerequisites**

- `pc-chain-config.json`

#### start-node
The `start-node` wizard starts a partner chain node

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
- **committee selection**: Provides a Cardano-based committee selection using the Ariadne algorithm.
- **Cardano-based block production rewards**: Calculation of rewards for Partner Chain block producers
and their Cardano delegators.

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
