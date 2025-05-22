# Partner chains setup

Partner Chains provides text based CLIs, for setting up a partner chain and starting nodes, called wizards.
In the [chain builder](./chain-builder.md), [permissioned node owner](./permissioned.md), and [registered node owner](./registered.md) guides these wizards are used.

This document explains what is performed by those wizards and how to perform such operations manually.

For dependency setup please see the respective guide for your desired setup.

## Generating keys

`<node-executable> wizards generate-keys` creates 3 specific keys, stores them in the keystore and also creates a JSON file with public keys.
These 3 keys are:
* Aura, sr25519 scheme key, with "key-type" `aura`
* GRANDPA, ed25519 scheme key, with "key-type" `gran`
* cross-chain, ecdsa scheme key, with "key-type" `crch`

For each of these key commands that are two commands invoked by wizard.
The first one is
```bash
<node-executable> key generate --scheme <scheme> --output-type json
```
, that has `secretPhrase` and `publicKey` fields in the output.
Please do save these outputs as they are needed in the subsequent steps.

The second one is
```bash
<node-executable> key insert --keystore-path <base-path>/keystore --scheme <scheme> --key-type <key-type> --suri <secretPhrase>
```
that stores the generated key in the keystore.

Commands above serve as instructions for generating and inserting required key in the keystore.
If there is a need for keys of different schema or type, or in different location, the instructions above should be used to insert it.

The wizard can also generate node network key, using following commands
```bash
mkdir -p <base-path>/network
mkdir key generate-node-key --file <base-path>/network/secret_ed25519
```

Please note that running node will require using `--base-path`, `--keystore-path` and `--node-key-file` to match the layout used above.

## Prepare configuration

`<node-executable> wizards prepare-configuration` sets up partner chain governance on the main chain and computes additional parameters that are used in later steps of the setup.

### Establishing partner chain on Cardano

Each partner chain is identified by the Genesis UTXO and it has some governance.
Use `<node-executable> smart-contracts governance init -c <GENESIS_UTXO> -g <GOVERNANCE_AUTHORITY> -t <THRESHOLD> -k <PAYMENT_KEY_FILE>`
command to spend the UTXO that will become the Genesis UTXO of your chain.
The Genesis UTXO has to be spendable by the payment key.
This command initializes governance of the chain as M of N multisig parametrized by GOVERNANCE_AUTHORITY and THRESHOLD parameters.

Note: for more details please read [smart-contracts commands documentation](../../toolkit/smart-contracts/commands/readme.md).

### Discover smart-contracts addresses and script hashes

Use `<node-executable> smart-contracts get-scripts -c <GENESIS_UTXO>` to obtain Cardano addresses and scripts hashes.
This guide assumes that the output is saved to `addresses.json` file.

### Establish bootnodes

The wizard prepares one bootnode address that is derived from the generated node network key and user input.
This guide does not cover bootnodes as it is not a partner chains specific topic.
Partner chains do not add any requirements nor capabilities in regards to bootnodes.

## Create chain-spec file

`<node-executable> wizards create-chain-spec` uses data from the previous step to generate chain-spec file.
It is important to understand that most of chain-spec file generation is delegated to `<node-exectuable>` itself.
Specifically, the wizard sets some specific environment variables and assumes that `<node-executable> build-spec` will use and that it will not require any other variables.
Because what `<node-exectuable> build-spec` uses and requires is finally in control of the chain builder, the following command can only be seens as an example:
```bash
export COMMITTEE_CANDIDATE_ADDRESS=$(jq -r '.addresses.CommitteeCandidateValidator' addresses.json)
export D_PARAMETER_POLICY_ID=$(jq -r '.policyIds.DParameter' addresses.json)
export PERMISSIONED_CANDIDATES_POLICY_ID=$(jq -r '.policyIds.PermissionedCandidates' addresses.json)
export ILLIQUID_SUPPLY_VALIDATOR_ADDRESS=$(jq -r '.addresses.IlliquidCirculationSupplyValidator' addresses.json)
export NATIVE_TOKEN_POLICY_ID='0x00000000000000000000000000000000000000000000000000000000'
export NATIVE_TOKEN_ASSET_NAME=''
```
Native token policy and asset name are out of partner chains control, they should be known by the chain builder.
For chains that do not intent to use them, it is assumed that `<node-exectuable> build-spec` will not need them.

Run `<node-executable> build-spec --disable-default-bootnode > chain-spec.json`.
As stated above, the content of the file depends mostly on `<node-executable>`.

Please consult our [intro doc chain-spec](../intro.md#chain-spec.json) section and update all the required fields, most notably:
* `genesisUtxo` and `slotsPerEpoch` of Sidechain Pallet
* `initialValidators` of Partner Chains Session Pallet
* `initialAuthorities` of Session Validator Management Pallet
* all occurrances `mainChainScripts`

## Setup D-parameter and permissioned Permissioned Candidates

`<node-executable> wizards setup-main-chain-state` guides user through creating (and conditionally submitting) two transactions that set up smart-contracts state.

The first one sets the D-parameter.
`<node-executable> smart-contracts upsert-d-parameter` is the lower level command that can be used for it.

The second one sets the permissioned candidates.
`<node-executable> smart-contracts upsert-permissioned-candidates` is the command to set permissioned candidates.

For more details, please refer to [smart-contracts commands documentation](../../toolkit/smart-contracts/commands/readme.md).

Please note that partner chains committee selection feature will use this data only after two Cardano epoch boundaries.
If these transactions were submitted in the epoch N, then committee selection will use this data from epoch N+2.

## Running partner chains node

`<node-executable> wizards start-node` sets up environment variables required for following Cardano and runs `<node-executable> --validator --chain chain-spec.json --base-path <BASE_PATH> --keystore-path <KEYSTORE_PATH> --node-key-file <NODE_KEY_FILE> --port <WSPORT> <BOOTNODES_PARAMETERS>`.

For setting the environment variables please consult [the documentation here](../intro.md#environment-variables).

Other parameters are regular substrate node parameters.
Use them accordingly to your setup.

## Register as committee candidate

For dependencies setup and broader context please read [registered guide](./registered.md).

This guide presents commands that could be used instead of `register1`, `register2`, and `register3` wizards.

### Getting signatures

Registration requires posting a message containing signatures to Cardano.
These signatures prove that an SPO wants to be regarded as committee candidate for given partner chain.
Use:
```bash
<node-executable> registration-signatures \
  --genesis-utxo <GENESIS_UTXO> \
  --mainchain-signing-key <STAKE_POOL_OPERATOR_SIGNING_KEY> \
  --sidechain-signing-key <PARTNER_CHAIN_SIGNING_KEY> \
  --registration-utxo <REGISTRATION_UTXO>
```
* GENESIS_UTXO is the UTXO that identifies a partner chain
* STAKE_POOL_OPERATOR_SIGNING_KEY is the `cborHex` without the first 4 characters of _StakePoolSigningKey_ed25519_ key file.
This is cold key, therefore this command is intented to be used on an offline machine.
* PARTNER_CHAIN_SIGNING_KEY is hex of the ecdsa key created the first step of this guide, `secretSeed` field of the `key generate` output.
* REGISTRATION_UTXO is a UTXO that SPO is able to spent when posting the message with signature. It prevents replay attacks.

The command outputs a JSON with following fields:
* `spo_public_key` - derived from STAKE_POOL_OPERATOR_SIGNING_KEY
* `spo_signature` - signature of a _registration message_ made with STAKE_POOL_OPERATOR_SIGNING_KEY signing key
* `sidechain_public_key` - derived from PARTNER_CHAIN_SIGNING_KEY
* `sidechain_signature` - signature of a _registration message_ made with STAKE_POOL_OPERATOR_SIGNING_KEY signing key

Note: the _registration message_ is composed of the genesis UTXO, sidechain public key, and registration UTXO.

### Submitting registration

Having this data obtained on an offline machine, it should be used on an online one to submit registration transaction to Cardano.
Use the `<node-executable> smart-contracts register` command.
There are two parameters requiring explanation:
* `partner-chain-signature` - use `sidechain_signature` field of `registration-signatures`
* `partner-chain-public-keys` - use `<PARTNER_CHAIN_SIGNING_KEY>:<AURA_KEY>:<GRANDPA_KEY>`,
where AURA_KEY and GRANDPA_KEY are obtained in same way as PARTNER_CHAIN_SIGNING_KEY was obtained for the `registration-signatures` command.

After this command is executed registration should become "effective" after two Cardano epochs boundaries.
`<node-executable> registration-status --mainchain-pub-key <SPO_PUBLIC_KEY> --mc-epoch-number <CARDANO_EPOCH_NUMBER> --chain chain-spec.json`
can be used to see if according to a partner chain the registration is valid.
Before running this command environment variables required by Cardano observability layer should be set, like when [running partner chains nodes](#running-partner-chains-node).
