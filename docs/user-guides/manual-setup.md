# Partner chains setup

Partner chains provide text based CLIs for setting up partner chain and starting nodes called wizards.
In [chain builder](./chain_builder.md), [permissioned node owner](./permissioned.md), and (registered node owner)[./registered.md] manuals these wizards are used.

This document explains what is performed by wizards and how perform such operations manually.

## Generating keys

`<node-executable wizards generate-keys` creates 3 specific keys, stores them in the keystore and also creates a JSON file with public keys.
These 3 keys are:
* Aura, sr25519 scheme key, with "key-type" `aura`
* GRANDPA, ed25519 scheme key, with "key-type" `gran`
* cross-chain, ecdsa scheme key, with "key-type" `crch`

For each of these keys commands that are two commands invoked by wizard.
The first is
```bash
<node-executable> key generate --scheme <scheme> --output-type json
```
, that has `secretPhrase` and `publicKey` fields in the output.
The second is
```bash
<node-executable> key insert --keystore-path <base-path>/keystore --scheme <scheme> --key-type <key-type> --suri <secretPhrase>
```
that stores generated key in a keystore.

Commands above serve as recipe for generating and inserting required key in the keystore.
Will there be a need for key of different schema or type or in different location, it should now be easy to get it.

The wizard can also generate node network key, using following commands
```bash
mkdir -p <base-path>/network
mkdir key generate-node-key --file <base-path>/network/secret_ed25519
```

Please note that running node will require using `--base-path`, `--keystore-path` and `--node-key-file` to match the layout used above.

## Prepare configuration

`<node-executable wizards prepare-configuration` sets up partner chain governance on the main chain and computes additional parameters that are used in later steps of the setup.

### Establishing partner chain on Cardano

Each partner chain is identified by the Genesis UTXO and it has some governance.
Use `<node-executable> smart-contracts governance init -c <GENESIS_UTXO> -g <GOVERNANCE_AUTHORITY> -t <THRESHOLD> -k <PAYMENT_KEY_FILE>`
command to spend the UTXO that will become the Genesis UTXO of your chain.
The Genesis UTXO has to be spendable by the payment key.
This command initializes governance of the chain as M of N multisig parametrized by GOVERNANCE_AUTHORITY and THRESHOLD parameters.

Note: for more details please read (smart-contracts documentation)[../../toolkit/smart-contracts/readme.md]

### Discover smart-contracts addresses and script hashes

Use `<node-executable> smart-contracts get-scripts -c <GENESIS_UTXO>` to obtain Cardano addresses and scripts hashes.
This guide assumes that the output is saved to `addresses.json` file.

### Establish bootnodes

Wizard perpares one bootnode address that is derived from the generated node network key and user input.
This guide does not cover bootnodes as not partner chains specific.
Partner chains do not add any bootnode requirements nor capabilities.

## Create chain-spec file

`<node-executable wizards prepare-configuration` uses data from the previous step to generate chain-spec file.
It is important to understand that most of chain-spec file generation is delegated to `<node-exectuable>` itself.
Specifically, wizards set some specific environment variables and assumes that `<node-exectuable> build-spec` will use them and will not require any other variables.
Because what `<node-exectuable> build-spec` uses and requires is finally in control of the chain developer, the following command can only be seens as an example:
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

Please consult
