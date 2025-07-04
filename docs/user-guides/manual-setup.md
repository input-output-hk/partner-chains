# Partner chains setup

Partner Chains provides text based CLIs, for setting up a partner chain and starting nodes, called wizards.
These wizards are used in the [chain builder](./chain-builder.md), [permissioned node owner](./permissioned.md), and [registered node owner](./registered.md) guides.

This document explains what these wizards do and how to perform these operations manually.

For dependency setup please see the respective guide for your desired setup.

## Generating keys

`pc-node wizards generate-keys` creates 3 specific keys, stores them in the keystore and also creates a JSON file with public keys.
These 3 keys are:
* Aura, sr25519 scheme key, with "key-type" `aura`
* GRANDPA, ed25519 scheme key, with "key-type" `gran`
* cross-chain, ecdsa scheme key, with "key-type" `crch`

For each of these keys, two commands are invoked by the wizard.
The first one is
```bash
pc-node key generate --scheme <scheme> --output-type json
```
, which outputs a JSON containing, among others, fields `secretPhrase` and `publicKey`.
Please do save these outputs as they are needed in the subsequent steps.

The second one is
```bash
pc-node key insert --keystore-path <base-path>/keystore --scheme <scheme> --key-type <key-type> --suri <secretPhrase>
```
that stores the generated key in the keystore.

Commands above serve as instructions for generating and inserting required key in the keystore.
If there is a need for keys of different schema or type, or in different location, the instructions above should be used to insert it.

The wizard can also generate node network key, using the following commands:
```bash
mkdir -p <base-path>/network
mkdir key generate-node-key --file <base-path>/network/secret_ed25519
```

Please note that running node will require using `--base-path`, `--keystore-path` and `--node-key-file` to match the layout used above.

## Prepare configuration

`pc-node wizards prepare-configuration` sets up partner chain governance on the main chain and computes additional parameters that are used in later steps of the setup.

### Establishing partner chain on Cardano

Each partner chain is identified by the Genesis UTXO and has a governance authority.
Use `pc-node smart-contracts governance init -c <GENESIS_UTXO> -g <GOVERNANCE_AUTHORITY> -t <THRESHOLD> -k <PAYMENT_KEY_FILE>`
command to spend the UTXO that will become the Genesis UTXO of your chain.
The Genesis UTXO has to be spendable by the payment key.
This command initializes governance of the chain as "at least M of N" multisig parametrized by GOVERNANCE_AUTHORITY and THRESHOLD parameters.
Where GOVERNANCE_AUTHORITY parameter is a list of N public key hashes, and THRESHOLD is M.

Example,
```
pc-node smart-contracts governance init \
  -c 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef#0 \
  -g aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc \
  -t 2 \
  -k payment-key.skey
```

Please read [governance guide](./governance/governance.md) for examples and explanation of the governance mechanism.


### Discover smart-contracts addresses and script hashes

Use `pc-node smart-contracts get-scripts -c <GENESIS_UTXO>` to obtain Cardano addresses and scripts hashes used by the partner chain smart contracts.
This guide assumes that the output is saved to `addresses.json` file.

Example:
```
pc-node smart-contracts get-scripts \
  --ogmios-url ws://localhost:1337 \
  -c 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef#0 \
  | jq .
{
  "addresses": {
    "CommitteeCandidateValidator": "addr_test1wr8w5nyqrqfz3fv4hwqdcqq6adplpg8wcq63p94xsemynyc5tpj9y",
    "DParameterValidator": "addr_test1wrgnr226dy6mfnz7439wk4mq4gmgpdp34s4rsgtn79r5xasy8384k",
    "GovernedMapValidator": "addr_test1wrgnr226dy6mfnz7439wk4mq4gmgpdp34s4rsgtn79r5xasy8384k",
    "IlliquidCirculationSupplyValidator": "addr_test1wzzdp0qhjjs7nt8rfll5gqdzmhnsd4z5sgrkdweumvhpf2g3uwvue",
    "PermissionedCandidatesValidator": "addr_test1wpj9s5hdd4x0257jxd45ttggms9vg2pyr8hy7f8p2crpwssmyg3qs",
    "ReserveValidator": "addr_test1wqz4dvwq4tt0qdt9fl7kuyw9nvdstqxjz788rrwqetwh0mgtafyf2",
    "VersionOracleValidator": "addr_test1wrfx2uvdnz32xsf908hygxryg5drup8ntxxfxm5re4rcaac6jujlq"
  },
  "policyIds": {
    "DParameter": "0x698f1cfa195610b54969095abc012cc70d4784aa2e41998024521419",
    "GovernedMap": "0xe65248ba058ae2a9ba122837a8d0a7aa9463bbb0f93318d66c551038",
    "PermissionedCandidates": "0x6b07de6b9e10ab0b7a93ed1392348403c6c4c68cf7f901b35fa5a7f0",
    "ReserveAuth": "0x83a5e4a89ecd9a343e98affcfd4c388526c54145439db2704b6abddf",
    "VersionOracle": "0xdf96ecb14bf361068ee3e86025d5810432704f8db0772640c10e7047"
  }
}
```
Please note that `smart-contracts` commands use Ogmios, in most examples `--ogmios-url` parameter is omitted and the default value is used.

Addresses depend on the Cardano network Ogmios is connected to.

### Establish bootnodes

The wizard prepares one bootnode address that is derived from the generated node network key and user input.
This guide does not cover bootnodes as it is not a partner chains specific topic.
Partner chains do not add any requirements nor capabilities in regards to bootnodes.

## Create chain-spec file

`pc-node wizards create-chain-spec` uses data from the previous step to generate chain-spec file.

It is important to understand that most of chain-spec file generation is delegated to `pc-node` itself.
Specifically, `pc-node build-spec` requires a set of environment variables.
The exact set of env vars is ultimately up to the chain builder, the following command can only be seen as an example:
```bash
export COMMITTEE_CANDIDATE_ADDRESS=$(jq -r '.addresses.CommitteeCandidateValidator' addresses.json)
export D_PARAMETER_POLICY_ID=$(jq -r '.policyIds.DParameter' addresses.json)
export PERMISSIONED_CANDIDATES_POLICY_ID=$(jq -r '.policyIds.PermissionedCandidates' addresses.json)
export ILLIQUID_SUPPLY_VALIDATOR_ADDRESS=$(jq -r '.addresses.IlliquidCirculationSupplyValidator' addresses.json)
export NATIVE_TOKEN_POLICY_ID='0x00000000000000000000000000000000000000000000000000000000'
export NATIVE_TOKEN_ASSET_NAME=''
```
Native token policy and asset name are out of partner chains control, they should be known by the chain builder.
For chains that do not intend to use them, it is assumed that `pc-node build-spec` will not need them.

Run `pc-node build-spec --chain template --disable-default-bootnode > chain-spec.json`.
As stated above, the content of the file depends mostly on `pc-node`.

Please consult our [intro doc chain-spec](../intro.md#chain-spec.json) section and update all the required fields, most notably:
* `genesisUtxo` and `slotsPerEpoch` of Sidechain Pallet
* `initialValidators` of Partner Chains Session Pallet
* `initialAuthorities` of Session Validator Management Pallet
* all occurrances of `mainChainScripts`

Note: reliance on env vars to build chain-spec is deprecated.
Demo node in this repository will eventually stop using them.
The `chain-spec` command will output genesis config with default/empty values that should be updated by external tools.

## Setup D-parameter and permissioned Permissioned Candidates

`pc-node wizards setup-main-chain-state` sets up D-parameter and Permissioned Candidates list on Cardano.
Please read this section to see how to do it using `smart-contracts` commands.

The first one creates a transaction to set the D-parameter. Example:
```
pc-node smart-contracts upsert-d-parameter \
  -c f8fbe7316561e57de9ecd1c86ee8f8b512a314ba86499ba9a584bfa8fe2edc8d#0 \
  --permissioned-candidates-count 7 \
  --registered-candidates-count 5 \
  -k PKA.skey | jq .
  ...
  {
    "transaction_to_sign": {
      "temporary_wallet": ...,
      "tx":{
        "cborHex": "84aa...redacted...f5f6",
        "description": "",
        "type": "Tx ConwayEra"
      },
      "tx_name":"Insert D-parameter"
    }
  }
```
Transaction present in the output should be singed and submitted as per [governance guide](./governance/governance.md).

The second one, `smart-contracts upsert-permissioned-candidates`, sets the permissioned candidates.
```
pc-node smart-contracts upsert-permissioned-candidates \
  -c f8fbe7316561e57de9ecd1c86ee8f8b512a314ba86499ba9a584bfa8fe2edc8d#0 \
  --permissioned-candidates-file permissioned_candidates.csv \
  -k PKA.skey | jq .
```
where permissioned_candidates.csv contains sets of keys of each permissioned candidate:
```
020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1:d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d:88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee
0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27:8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48:d17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69
```
The first key in a row is partner chain public key, the second one is Aura public key, the last one is GRANDPA public key.

Please note that partner chains committee selection feature will use this data only after two Cardano epoch boundaries.
If these transactions were submitted in the epoch N, then committee selection will use this data starting from epoch N+2.

## Running partner chains node

`pc-node wizards start-node` sets up environment variables required for following Cardano and runs:
```bash
pc-node --validator \
    --chain chain-spec.json \
    --base-path <BASE_PATH> \
    --keystore-path <KEYSTORE_PATH> \
    --node-key-file <NODE_KEY_FILE> \
    --port <WSPORT> \
    <BOOTNODES_PARAMETERS>
```

For setting the environment variables please consult [the documentation here](../intro.md#environment-variables).

Other parameters are regular substrate node parameters.
Use them accordingly for your setup.

## Register as committee candidate

For dependencies setup and broader context please read [registered guide](./registered.md).

This guide presents commands that could be used instead of `register1`, `register2`, and `register3` wizards.

### Getting signatures

Registration requires posting a message containing signatures to Cardano.
These signatures prove that an SPO is the owner of the keys being registered,
and wants to be considered for committee membership for a given partner chain.
Use:
```bash
pc-node registration-signatures \
  --genesis-utxo <GENESIS_UTXO> \
  --mainchain-signing-key <STAKE_POOL_OPERATOR_SIGNING_KEY> \
  --sidechain-signing-key <PARTNER_CHAIN_SIGNING_KEY> \
  --registration-utxo <REGISTRATION_UTXO>
```
* GENESIS_UTXO is the UTXO that identifies a partner chain
* STAKE_POOL_OPERATOR_SIGNING_KEY is the `cborHex` without the first 4 characters of _StakePoolSigningKey_ed25519_ key file.
This is cold key, therefore this command is intented to be used on an offline machine.
* PARTNER_CHAIN_SIGNING_KEY is hex of the ecdsa key created during the first step of this guide (the `secretSeed` field of the `key generate` output).
* REGISTRATION_UTXO is a UTXO that SPO is able to spend when posting the message with signature. It prevents replay attacks.

The command outputs a JSON with following fields:
* `spo_public_key` - derived from STAKE_POOL_OPERATOR_SIGNING_KEY
* `spo_signature` - signature of a _registration message_ made with STAKE_POOL_OPERATOR_SIGNING_KEY signing key
* `sidechain_public_key` - derived from PARTNER_CHAIN_SIGNING_KEY
* `sidechain_signature` - signature of a _registration message_ made with STAKE_POOL_OPERATOR_SIGNING_KEY signing key

Note: the _registration message_ is composed of the genesis UTXO, sidechain public key, and registration UTXO.

### Submitting registration

Having this data obtained on an offline machine, it should be used on an online one to submit the registration transaction to Cardano.
Use the `pc-node smart-contracts register` command.
There are two parameters requiring explanation:
* `partner-chain-signature` - use `sidechain_signature` field of `registration-signatures` command output,
* `partner-chain-public-keys` - use `<PARTNER_CHAIN_SIGNING_KEY>:<AURA_KEY>:<GRANDPA_KEY>`,
where AURA_KEY and GRANDPA_KEY are obtained in same way as PARTNER_CHAIN_SIGNING_KEY was obtained for the `registration-signatures` command.

After this command is executed registration should become "effective" after two Cardano epoch boundaries.
`pc-node registration-status --mainchain-pub-key <SPO_PUBLIC_KEY> --mc-epoch-number <CARDANO_EPOCH_NUMBER> --chain chain-spec.json`
can be used to see if according to a partner chain the registration is valid.
Before running this command environment variables required by Cardano observability layer should be set, like when [running partner chains nodes](#running-partner-chains-node).
