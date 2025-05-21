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
  * [Configuration](#configuration)
    * [`chain-spec.json`](#chain-spec.json)
    * [Environment Variables](#environment-variables)
    * [Keys](#keys)
  * [Wizards](#wizards)
    * [generate-keys](#generate-keys)
    * [prepare-configuration](#prepare-configuration)
    * [create-chain-spec](#create-chain-spec)
    * [setup-main-chain-state](#setup-main-chain-state)
    * [start-node](#start-node)
    * [register1](#register1)
    * [register2](#register2)
    * [register3](#register3)
    * [deregister](#deregister)
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

### Configuration
Partner chains are configured through a combination of configuration files and environment
variables. On top of the files outlined below, the command line wizard provides a convenient layer
for initializing configurations.

#### Chain spec

The chain spec file contains basic chain ifnormation, initial operational parameters of the network,
and genesis configuration for all pallets present in the initial runtime.
Read the [official documentation](https://docs.polkadot.com/develop/parachains/deployment/generate-chain-specs/)
for more information about the chain spec file itself. For the sake of configuring the Partner Chain
toolkit components, only the genesis configuration (located in `.genesis.runtimeGenesis.config`) of
Partner Chain pallets is relevant.

Below is an overview of how various Partner Chain toolkit's pallets can be configured. Note that,
because Substrate leaves it up to the developer to name individual pallets in their runtime, the
names of top-level configuration fields may be different. This document assumes that pallets in
the runtime use their canonical names, eg. the Governed Map pallet is called `GovernedMap` in
the runtime, resulting in the config field `governedMap` etc.

##### Obtaining main chain scripts

Some of the pallets need to be configured with _main chain scripts_, that is Cardano script addresses
and hashes that are needed for Cardano observability components to correctly locate data in the Cardano
ledger. These parameters can be obtained using the genesis UTXO of the Partner Chain by executing the
`get-scripts` command provided together with other Partner Chain smart contracts offchain commands:
```shell
partner-chains-demo-node smart-contracts get-scripts --genesis-utxo $GENESIS_UTXO
```
The command output might look like this:
```json
{
  "addresses": {
    "CommitteeCandidateValidator": "addr_test1wpwy4z3nhjs9vqx9j204dclj85sg44m2n6wwgqgmd95gees5elx7z",
    "DParameterValidator": "addr_test1wr7jhcgaf54cqfezl40g4qraq3uzq72qp8g05sfwcyd5u0ctql5kn",
    "GovernedMapValidator": "addr_test1wr7jhcgaf54cqfezl40g4qraq3uzq72qp8g05sfwcyd5u0ctql5kn",
    "IlliquidCirculationSupplyValidator": "addr_test1wrddtqdhyneqtwsc8tguce7tc4cz4rvgttpz02el2gxgqqckcx4y9",
    "PermissionedCandidatesValidator": "addr_test1wzwnujc0vpgnckyea4yrndzvk2szkm7jhgx6ecpp2kjgzdqnk6la5",
    "ReserveValidator": "addr_test1wrgw9l32rscccuwzlhu38306wy0ep3ea8hlprq9xayw3e3cp8e7m6",
    "VersionOracleValidator": "addr_test1wz7a4msuddhgc8yca6geyx9y2hp0grl5qpnall2phlpqvggllswwc"
  },
  "policyIds": {
    "DParameter": "0x127d554550b46ada9e2b6fce4f0a9be18ad6a3b15e163638c4c207a6",
    "GovernedMap": "0x94ea0975ddb667dfc8b567ff8f9ce1d6efb1d2dabfd57577e7f68680",
    "PermissionedCandidates": "0x0e025dfd43feae1cdf95049a59a3b00d566901ffe5a1ecdff738e80d",
    "ReserveAuth": "0x3a2883d03e1321c443e4070397c2c10d03f98913c09197acd61be326",
    "VersionOracle": "0x5f15fc15749d80f6bfac3ada9153723b5f528f2ecd461ceaa1fef0c5"
  }
}
```

##### Sidechain pallet

The sidechain pallet is especially important due to all other Partner Chains toolkit pallets depending
on it and the fact that the values in its genesis config are immutable throughout the lifetime of a
single Partner Chain. These values are:
- `genesisUtxo`:
  the genesis UTXO of the Partner Chain, which is used as its idenifier in the Partner Chains ecosystem
  and has various other uses in the chain's operation
- `slotsPerEpoch`:
  the number of slots per Partner Chain epoch. This value can be arbitrarily chosen, provided that the
  resulting Partner Chain epochs can't cross Cardano epoch boundary. Keep in mind that other Partner
  Chains toolkit features may not work properly if Partner Chain epochs are very short.

For example, the sidechain pallet can be configured like this:
```json
{
  "sidechain": {
    "genesisUtxo": "c14edd7764339d9877f76259184ecebca240e8cdf41b1a837c34637e7d50b5ed#0",
    "slotsPerEpoch": 60
  }
}
```

##### Session pallets

The Partner Chains toolkit uses its own version of the stock Session pallet. This
pallet must be configured with the initial block producing committee, by adding
the block producers' keys under `initialValidators`. The exact number and types of
keys to add depend on the validator ID type and session keys used by the Partner
Chain being configured.

Assuming the chain is using Substrate's default 32-bit account ID and Aura and
Grandpa for its consensus, the configuration may look like the following:
```json
{
  "session": {
    "initialValidators": [
      [
        "5C7C2Z5sWbytvHpuLTvzKunnnRwQxft1jiqrLD5rhucQ5S9X",
        {
          "aura": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
          "grandpa": "5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu"
        }
      ]
    ]
  }
}
```

Keep in mind that for compatibility with Grandpa, a Partner Chain needs to also use
the stock Session pallet provided by Substrate. As this pallet is no longer
responsible for managing session committee, its genesis configuration
should be left empty, ie.:
```json
{
  "palletSession": {
    "keys": [],
    "nonAuthorityKeys": []
  }
}
```

Similarly, if a Partner Chain uses Aura and Grandpa for consensus, their genesis
configuration should be left empty, since the session pallet is responsible for
managing their validator sets, ie.:
```json
{
  "aura": {
    "authorities": []
  },
  "grandpa": {
    "authorities": []
  }
}
```

##### Session Committee Management pallet

This pallet builds on top of the session pallet and allows a Partner Chain's block
producing committees to be selected based on registrations and parameters set in
its Cardano main chain's ledger. It is responsible for rotating sessions based on
data observed on Cardano and storing configuration for the observability data
source. Its genesis configuration includes:
- `initialAuthorities`:
  The initial authority set. The entries on this list should correspond to those
  set in `initialValiadors` of the Session pallet, and consist of two components:
  the validator's cross-chain public key and validator's session keys. The session
  keys depend on the key set used by the consensus mechanism employed by the Partner
  Chain and should have the same values as those in `initialValidators`. The
  cross-chain public key, on the other hand, is distinct from the validator ID
  used by the Session pallet.
- `mainChainScripts`:
  Cardano address and policy IDs which are used by the observability data source to
  locate committee selection input data in the Cardano ledger. These are:
  - `committee_candidate_address`: The Cardano address containing the Cardano
    SPO registrations.
  - `permissioned_candidates_policy_id`: Minting policy ID needed to read the
    list of permissioned block producers.
  - `d_parameter_policy_id`: Minting policy ID that is necessary to read the
    D-Param for the Ariadne selection algorithm.
  It is crucial to correctly configure these parameters, as they are necessary for
  the next committee to be selected after the chain's launch.

For example, assuming the chain uses Aura and Grandpa, a genesis configuration can
look like the following:
```json
{
  "sessionCommitteeManagement": {
    "initialAuthorities": [
      [
        "KW39r9CJjAVzmkf9zQ4YDb2hqfAVGdRqn53eRqyruqpxAP5YL",
        {
          "aura": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
          "grandpa": "5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu"
        }
      ]
    ],
    "mainChainScripts": {
      "committee_candidate_address": "addr_test1wpwy4z3nhjs9vqx9j204dclj85sg44m2n6wwgqgmd95gees5elx7z",
      "d_parameter_policy_id": "0x127d554550b46ada9e2b6fce4f0a9be18ad6a3b15e163638c4c207a6",
      "permissioned_candidates_policy_id": "0x0e025dfd43feae1cdf95049a59a3b00d566901ffe5a1ecdff738e80d"
    }
  },
}
```

##### Governed Map pallet

The Governed Map stores the main chain scripts used by its data source when
observing changes to the key-value pairs stored on Cardano. These scripts are:
- `asset_policy_id`: minting policy used to secure the key-value store
- `validator_address`: Cardano address at which the key-value pairs are kept
These are configured as in the following example:
```json
{
  "governedMap": {
    "mainChainScripts": {
      "asset_policy_id": "0x94ea0975ddb667dfc8b567ff8f9ce1d6efb1d2dabfd57577e7f68680",
      "validator_address": "addr_test1wr7jhcgaf54cqfezl40g4qraq3uzq72qp8g05sfwcyd5u0ctql5kn"
    }
  }
}
```

The Governed Map pallet can be configured after the chain launch as well. In that
case, the `mainChainScripts` field can be set to `null` to leave it unconfigured.

##### Native Token Management pallet

The Native Token Management pallet handles information about native tokens released
from Cardano to the Partner Chain. For the feature to function, it must be
configured with the following main chain scripts:
- `illiquid_supply_validator_address`: the Cardano address at which tokens are
  locked so they can be received on the Partner Chain.
- `native_token_asset_name` and `native_token_policy_id`: the asset name and policy
  ID of the Partner Chain's native token.

```json
{
  "nativeTokenManagement": {
    "mainChainScripts": {
      "illiquid_supply_validator_address": "addr_test1wrddtqdhyneqtwsc8tguce7tc4cz4rvgttpz02el2gxgqqckcx4y9",
      "native_token_asset_name": "0x5043546f6b656e44656d6f",
      "native_token_policy_id": "0xada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4"
    }
  }
}
```

#### Environment Variables

Some of the configuration values used by the Partner Chain toolkit components are
read from the local environment. These generally come in two categories:
1. operational data used by the data sources
2. genesis configuration. Some Partner Chains may read them from environment when
   run without a chain spec file or when generating a new one.

##### Data source configuration

The Db-Sync based data sources provided by the Partner Chains toolkit require a
connection to a Postgres database fed by Db-Sync to query Cardano data. For this
a connection string should be provided when running a Partner Chain's node in the
`DB_SYNC_POSTGRES_CONNECTION_STRING` environment variable, eg.:
```sh
export DB_SYNC_POSTGRES_CONNECTION_STRING="postgres://postgres-user:password@localhost/db-sync-db"
```

Some data sources also expect environment variables that provide data on network
parameters used by the Cardano main chain:
- `CARDANO_SECURITY_PARAMETER`: the *k* parameter that determines how many
  confirmations are required for a block to be considered stable. Needs to be set
  to the value of `securityParam` as configured in the shelley-genesis config file
  passed to `cardano-node` via the `--shelley-genesis` parameter,
- `CARDANO_ACTIVE_SLOTS_COEFF`: The fraction of slots in which a block is expected
  to be produced on Cardano. Needs to be set to the value of `activeSlotsCoeff` as
  configured in the shelley-genesis config file passed to `cardano-node` via the
  `--shelley-genesis` parameter,

as well as information on Cardano slot and epoch configuration:

- `MC__SLOT_DURATION_MILLIS`: duration of a Cardano slot in millis
- `MC__EPOCH_DURATION_MILLIS`: duration of a Cardano epoch in millis
- `MC__FIRST_EPOCH_TIMESTAMP_MILLIS`, `MC__FIRST_EPOCH_NUMBER` and
  `MC__FIRST_SLOT_NUMBER`: these parameters must indicate the point in time after
  the start of the Shelley Cardano era, after which the two other parameters above
  have not changed anymore. Typically, these values should point to the beginning
  of the Shelley era, but for chains that modified their slot or epoch duration
  after that, the start of any Cardano epoch after the last change could be used,
  as long as it is before any Partner Chains smart contracts have been deployed

Lastly, every Partner Chain node needs the `BLOCK_STABILITY_MARGIN` variable to
be set. This variable determines an additional margin added to
`CARDANO_SECURITY_PARAMETER` when selecting the latest stable Cardano block to be
included in the block header. This value should normally be set to `0`, but `1`
can be used if the network experiences a high number of blocks rejected because of
Db-Sync lag. Values higher than `1` should not be used in general.

##### Genesis configuration environment variables

Some Partner Chains may choose to read genesis configuration for some pallets from
the environment when running a node without a chain spec, or generating a new spec.
The list of these pallets and their environment variables is presented below. The
names of the enviroment variables correspond directly to the names of genesis
config fields.

###### Sidechain
- `GENESIS_UTXO`: genesis UTXO of the Partner Chain
- `SLOTS_PER_EPOCH`: number of slots per Partner Chain epoch

###### Committee selection
- `COMMITTEE_CANDIDATE_ADDRESS`: Cardano address of SPO registrations
- `D_PARAMETER_POLICY_ID`: ID of the minting policy used for the D-Parameter
- `PERMISSIONED_CANDIDATES_POLICY_ID`: ID of the minting policy used for the
  permissioned candidates

###### Native Token Management
- `NATIVE_TOKEN_POLICY_ID` and `NATIVE_TOKEN_ASSET_NAME`: Minting policy and asseet
  name of the Cardano native token of the Partner Chain
- `ILLIQUID_SUPPLY_VALIDATOR_ADDRESS`: Cardano address at which tokens are locked
  before being available on the Partner Chain

#### Keys
The partner chain node needs three different keys to be present in a Substrate key store accessible
by the partner chain node.
1. `sidechain_pub_key`: The sidechain public key (ECDSA)
1. `aura_pub_key`: The aura public key (sr25519)
1. `grandpa_pub_key`: The grandpa public key (Ed25519)

By default the partner chain node process will look for key stores in the base path directory. Refer to the [official Polkadot  guide](https://docs.polkadot.com/infrastructure/running-a-validator/onboarding-and-offboarding/key-management/)
or your particular Partner Chain's documentation for information on how
to manage your node keys.

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

**Running the wizard**

```shell
$ pc-node wizards generate-keys
```

**Output Files**

- `pc-resources-config.json` Contains basic networking, ports and path settings
- `partner-chains-public-keys.json` Contains aura, grandpa and sidechain public keys


#### prepare-configuration
The `prepare-configuration` wizard will guide you through the configuration needed to create a
governance authority. The main output of this wizard is the `pc-config.json` file.

**Prerequisites**

- The `ogmios` service must be up and running
- The wizard will ask for the payment signing key file `payment.skey`.
- The payment address must be funded

Please refer to the `cardano-cli`
[documentation](https://developers.cardano.org/docs/get-started/cardano-cli/get-started/#generate-a-payment-key-pair-and-an-address)
for details on how to create keys.

**Running the wizard**

```shell
$ pc-node wizards prepare-configuration
```

**Output**

- `pc-chain-config.json` Contains partner chain properties
- `pc-resources-config.json` Contains basic networking, ports and path settings



#### create-chain-spec

The `create-chain-spec` wizard creates a chain specification based on an existing
`pc-chain-config.json` file (which can be generated using the `prepare-configuration` wizard).
The resulting chain specification file is ready to be distributed to block production committee candidates.

**Prerequisites**

- `pc-chain-config.json` Contains partner chain properties
- `pc-resources-config.json` Contains basic networking, ports and path settings

**Running the wizard**

```shell
$ pc-node wizards create-chain-spec
```

**Output**

- `chain-spec.json` Substrate chain specification


#### setup-main-chain-state

The `setup-main-chain-state` wizard configures the D-parameter and permissioned candidates list on the main chain.

:warning: These operations require transaction fees, so the payment key must be funded with ADA.

**Prerequisites**

- `pc-chain-config.json` Contains partner chain properties
- `chain-spec.json` Substrate chain specification
- The payment key address must be funded

**Running the wizard**
```shell
$ pc-node wizards setup-main-chain-state
```

#### start-node
The `start-node` wizard starts a partner chain node

**Prerequisites**
- `pc-chain-config.json` Contains partner chain properties
- `chain-spec.json` Substrate chain specification

**Running the wizard**
```shell
$ pc-node wizards start-node
```

#### register1

The `register1` wizard is the first out of three steps in registering a node as committee
candidate. The wizard will prompt users to select a UTXO which is going to be consumed in the
registration process that follows.

After selecting a UTXO the wizard will print a `register2` wizard command for generating
signatures. This command should be executed on an offline machine to ensure that the Cardano
`cold.skey` (which will be required) is not exposed to the internet.

**Prerequisites**
- `pc-chain-config.json` Contains partner chain properties
- `chain-spec.json` Substrate chain specification
- The payment key address must be funded

**Running the wizard**
```shell
$ pc-node wizards register1
```

#### register2
The `register2` wizard is the second out of three steps in registering a node as committee candidate.
The wizard will use the user provided
[cold.skey](https://developers.cardano.org/docs/operate-a-stake-pool/cardano-key-pairs/#cardano-stake-pool-key-pairs)
to sign the registration message.

:warning: We suggest running the `register2` wizard on a separate machine that isn't connected to the internet.

Finally, the wizard will output a `register3` wizard invocation that should be executed on
the machine where `register1` was executed before to conclude the registration.

**Prerequisites**
- `cold.skey` A private Cardano stake pool signing key

**Running the wizard**
```shell
$ pc-node wizards register2 --genesis-utxo <GENESIS_UTXO>
      --registration-utxo <REGISTRATION_UTXO>
      --sidechain-pub-key <SIDECHAIN_PUB_KEY>
      --aura-pub-key <AURA_PUB_KEY>
      --grandpa-pub-key <GRANDPA_PUB_KEY>
      --sidechain-signature <SIDECHAIN_SIGNATURE>
```

:information_source: The actual values will be provided in the `register1` output.

#### register3
The `register3` wizard is the third and final step in registering a node as committee candidate.

**Prerequisites**
- `pc-chain-config.json` Contains partner chain properties
- `chain-spec.json` Substrate chain specification

**Running the wizard**
```shell
$ pc-node wizards register3 --genesis-utxo <GENESIS_UTXO>
	--registration-utxo <REGISTRATION_UTXO>
	--sidechain-pub-key <SIDECHAIN_PUB_KEY>
	--aura-pub-key <AURA_PUB_KEY>
	--grandpa-pub-key <GRANDPA_PUB_KEY>
	--sidechain-signature <SIDECHAIN_SIGNATURE>
```

:information_source: The actual values will be provided in the `register2` output.

#### deregister
The `deregister` wizard removes a candidate from the committee. The candidate is identified by the
genesis-utxo it was registered with.

**Prerequisites**
- `pc-chain-config.json` Contains partner chain properties
- `payment.skey` Funded payment key
- `cold.vkey` A public Cardano stake pool verification key

**Running the wizard**
```shell
$ pc-node wizards deregister
```

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
