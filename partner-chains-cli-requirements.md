# partner-chains-cli requirements

Vocabulary:

`cli` - short for `partner-chains-cli`

`chain config` - short for `partner-chains-cli-chain-config.json`, contains configuration of the
chain - every chain participant should have it the same.

`resources config` - short for `partner-chain-cli-resources-config.json`, contains resources
addresses, paths, etc. not every chain participant needs all of them, they are specific to user
setup.

Rule of thumb: when `cli` asks for a configuration value, it should propose value from config file,
if the value of config file is missing, it should propose another, perhaps hardcoded value.

Rule of thumb: when `cli` asks for a configuration value, it should explain what the value is used
for.

## `generate-keys` command (wizard)

Wraps generating keys and inserting them to keystores for the chain participants.
It is the first step to run by each network participant, who doesn't have keys yet.

Output: keys triplet in hex format, node id

effects:

* keystore created with keys triplet and the node key
* `partner-chains-public-keys.json` is created

* `cli` notifies users that it is going to generate `ecdsa` (sidechain key), `ed25519` (grandpa)
  and `sr25519` (aura) keys are required for the chain,
  and store them in the keystores required to run a node.
* `cli` reads `resources config` field `substrate_node_base_path`, and if it is missing, it asks
  user for it (proposed default is `data`)
* for each key type, `cli` checks if the keystore file exists in
  the `<substrate_node_base_path>/chains/partner_chains_template/keystore` directory,
  if it does, it asks user if they want to overwrite it (`y/N`),
	* if `y`, `cli` sets dummy values in the env that allows building template chain-spec (it is
      required because underlying polkadot-sdk code builds spec and reads the chain id from it),
	  and runs `partner-chains-node key generate --output-type json` and parses output
	* uses output `secretPhrase` as `--suri` passed to `partner-chains-node key insert`command
	* remembers `publicKey`
* `cli` notifies user about the absolute path of keystores
* `cli` outputs triplet of public keys in hex format in JSON format, and stores them
  to `partner-chains-public-keys.json` file.
  It notifies user that they can share these public keys with the chain governance authority, if
  they wish to be included as one of the initial permissioned candidates.
  Json format:

```json
{
	"sidechain_pub_key": "0x<key>",
	"aura_pub_key": "0x<key>",
	"grandpa_pub_key": "0x<key>"
}
```

* `cli` checks if the network key file is present
  at `<substrate_node_base_path>/chains/partner_chains_template/keystore/node_key`,
  and if it is valid
* if network key file is absent or not OK, it notifies user that it will generate ed25519 network
  key and store in the keystores directory,
  `cli` should use `partner-chains-node key generate-network-key` command for it.
* `cli` presents user their node id (obtained from the node key)

## 'prepare-configuration' command (wizard)

This command runs a wizard to complete `chain config` and `resources config` files.
Intended to be run by the governance authority (GA). It shares `chain config` file with the chain
participants.
`resources config` file is complemented by other wizards, when required.

### Establish bootnode step

Effect: `chain config` should have `bootnodes` array set with the bootnode of the chain.

* `cli` tries to inspect default node key
  file `<substrate_node_base_path>/chains/partner_chains_template/keystore/node_key` to obtain the
  node id
* if it fails to obtain the node id, it lectures the user that they should run `generate-keys`
  command first, and exits.
* `cli` asks the governance authority, if their bootnode will be accessible via hostname or IP
  address (`hostname`/`ip`)
* if `hostname`, then it asks for the hostname
* if `ip`, then it asks for the public IP
* `cli` updates `chain config` `bootnodes` array with the value constructed from hostname or IP and
  the node id
* `cli` lectures the user that they can manually modify the `chain config` file to edit `bootnodes`

### Establish sidechain parameters step

Effect: `chain config` should have `chain_parameters` part set and some of `resources config` as
well.

After the command is run, `cli` lectures user, that it reads data from `chain config`
and `resources config` files, and will use entries there as default values.

* `cli` reads `chain config` and `resources config` files.
  `cli` should not fail if the files are missing. If either has incorrect format,
  it should inform user that it has to be deleted of fixed manually, and exit.
  `cli` writes to user that it is going to establish partner chain parameters: `chain_id`
  and `governance_authority`.
  If `chain config` field `chain_parameters.governance_authority` is present,
  `cli` presents it to the user, and asks if it is correct (`y/N`).
  If user denies `N`, it is not correct, the following happens:
  * `cli` asks for the Cardano CLI command, proposing default from `resources config`
    field `cardano_cli`,
    if config is unavailable proposed default is `cardano-cli`.
    `cli` updates the `resources config` with user choice - this pattern applies to all configuration
    fields (will be skipped from now on)
  * `cli` asks for payment verification key file, proposing default from `resources config`
    field `cardano_payment_verification_key_file`,
    if the config is unavailable, then the proposed default is `payment.vkey`.
  * `cli` tries to read payment verification key and derive its
    hash: `<cardano-cli-command> address key-hash --payment-verification-key-file <cardano-payment-verfication-key-file>`
    If the command fails, `cli` informs the user about the failure and exits.
    Otherwise, it updates the `chain config` field `chain_parameters.governance_authority` with the
    key hash and notifies user about it.
* `cli` asks for the chain id, notifying user that pair `(governance authority, chain id)`identifies
  a partner chain,
  and it has to be unique, and that allowable values are in range [0; 65535], `chain config`
  field `chain_parameters.chain_id` is used as default (and target value), `0` is proposed as
  hardcoded default.
* `genesis_utxo`, `threshold_numerator`, `threshold_denominator` fields of object `chain_parameters`
  are set to `0000000000000000000000000000000000000000000000000000000000000000#0`, `2`, `3`
  respectively,
  without wizard support (used for legacy chains, like MN).
* `cli` starts `main chain configuration` step

### Establish main chain configuration step

Effect: `chain config` should have `cardano` object set with fields for
main chain
configuration: `security_parameter`, `active_slots_coeff`, `first_epoch_number`, `first_slot_number`, `epoch_duration_millis`, `first_epoch_timestamp_millis`,
main chain follower
configuration: `committee_candidates_address`, `d_parameter_policy_id`, `permissioned_candidates_policy_id`,

* `cli` presents trustless-sidechain version (from binary)
* `cli` asks for the Cardano network magic, lecturing user about `0` for `mainnet`, `1`
  for `preprod`, `2` for `preview`, using `chain config` field `cardano.network` as default,
  in absence of the field, the proposed default is `0`, for 0, 1, 2 it sets `cardano`
  parameters using hardcoded values, otherwise it asks for each of fields values, giving hints about expected content (
  example `first_epoch_number` is a number of the first epoch in shelley era).
* `cli` notifies user that for getting main chain follower configuration it needs `kupo`
  and `ogmios`, and asks for:
  `kupo.protocol` (http/https, default http), `kupo.hostname` (default localhost), `kupo.port` (
  default 1442), and for:
  `ogmios.protocol` (http/https, default http), `ogmios.hostname` (default
  localhost), `ogmios.port` (default 1337).
* `cli` runs `trustless-sidechain-cli addresses` command with all the required parameters taken
  from `resources config` and `chain config`, parses the response, and
  updates `committee_candidates_address`, `d_parameter_policy_id`, `permissioned_candidates_policy_id`
  fields of `cardano_addresses` of `chain config`, notifying user about their values.
* if `chain config` array `initial_permissioned_candidates` is absent, then `cli` sets it to empty
  array.
* `cli` exits notifying user that `chain config` is ready for being distributed to network
  participants
  and also that `create-chain-spec` should be executed when keys of permissioned candidates are
  gathered.

## `create-chain-spec` command (wizard)

This command generates chain spec using `chain config` and the node executable.

`partner-chains-node build-spec` without `--chain` parameter should be used, `cli` will lean
on default behavior of the binary, in order to not introduce a new parameter.

* `cli` reads `chain config` and presents to the user content of `chain_parameters`
  and `initial_permissioned_candidates`.
  Notifies user that it is going to create `chain-spec.json` file using these parameters.
* `cli` runs the `build-spec` command of node executable, and if it fails, it informs the user about the failure and
  exits.
* `cli` notifies user about path to the generated `chain-spec.json` file
* `cli` notifies user that if they are the governance authority, they should
  run `setup-main-chain-state` command

## setup-main-chain-state

Governance authority command.

* `cli` lectures users that their actions will cost ADA
* `cli` reads `chain config` and `resources config` files, if any values required for further step
  are missing,
  it lectures user to run `prepare-configuration` command first, and exits.
* `cli` reads permissioned candidates from `chain config` (or chain-spec) and from Cardano,
* in case of discrepancy, `cli` asks user if it should be updated (`Y/n`),
* if `Y`, `cli` uses `trustless-sidechain-cli` to update the permissioned candidates
* `cli` checks if D-parameter is present on the main chain
* if it is present, `cli` shows user the value and ask if it should be updated (`y/N`), if `y`:
*
	* `cli` asks for the new value of R (registered candidates seats) proposing 0 as default,
*
	* `cli` asks for the new value of P (permissioned candidates seats) proposing number of
	  permissioned candidates as default,
*
	* `cli` updated D-parameter on the main chain using `trustless-sidechain-cli` command

## `run-node` command (wizard)

* `cli` checks if all required keystores are present, if not it informs user that they should
  run `generate-keys` command first, and exits.
* `cli` checks if the chain-spec file is present, if not it informs user that they should obtain it
  from the governance authority or run `create-chain-spec` command first, and exits.
* `cli` reads the `chain config` and if it is missing or invalid, it informs user that they should
  obtain it from the governance authority or run `prepare-configuration` command first, and exits.
* `cli` reads `resources config` field `db_sync_postgres_connection_string`, only if it is missing (
  to avoid asking each time node is started), it asks for it, proposing
  default `postgresql://postgres-user:postgres-password@localhost:5432/cexplorer`
* unless `--silent` was used, `cli` outputs all relevant parameters and asks user if they are
  correct (`Y/n`), if `n`, it informs user that they should edit `chain config`
  and/or `resources config` files and run the command again.
* `cli` sets the environment variables required by the node, and runs the node
  using `partner-chains-node run` command.

Test what happens when user chooses utxo with some minimal amount of funds, to see if such utxo is
suitable or if we should filter them out.

## `register` wizard

Registration is a three-step process, with the second step executed on the cold machine, so there
are three wizards.

### `register-1` wizard

This part obtains registration utxo.

* `cli` checks if `chain config` is present and valid, if not it informs user that they should
  obtain it from the governance authority or run `prepare-configuration` command first, and exits.
* `cli` checks for sidechain keystore, using `resources config` base path, and if it is missing, it
  informs user that they should run `generate-keys` command first, and exits.
* `cli` runs steps as `prepare-configuration` command in order to establish: `cardano_cli`,
  and `cardano_payment_verification_key_file`
* `cli` asks for cardano-node socket path, proposing default from `resources config`
  field `cardano_node_socket_path`, config is unavailable, then the proposed default
  is `node.socket`.
* `cli` derives address from the payment verification
  key: `<cardano-cli-command> address build --payment-verification-key-file <cardano-payment-verification-key-file> <cardano-network-parameter>`
  `<cardano-network-parameter>` is `--testnet-magic <cardano-network>` if `cardano.network` is
  not `0`, otherwise it is empty.
  `cli` executes the command to read user
  utxos: `<cardano-cli-command> query utxo <cardano-network-parameter> --address <derived-address>`,
  parses it (alternative: use cardano-cli feature to output a json file),
  filters them, to only retain ones with TxOutDatumNone and presents them to the user, as a table
  where each row has a format `<row number>: "<utxo id>" "<amount>"`, and asks user to pick one,
  proposing `0` as default.
  If there are no suitable utxos, `cli` informs the user about it and exits.
  If `<cardano-cli-command>` fails, `cli` informs the user about the failure and exits.
  `cli` informs user to not spent chosen utxo, because it needs to be consumed later in the
  registration process.
* `cli` outputs the whole command for obtaining signatures (register-2 wizard) and informs user that
  they should run it on a machine with the mainchain cold signing key.

### `register-2` wizard

This part obtains signatures for the registration message.
All parameters, except SPO cold signing key, including sidechain signing key, are supplied as
arguments.

* `cli` notifies user that it will use SPO cold signing key for signing the registration message
* `cli` asks for the path for the mainchain cold signing key, proposing default from `cold.vkey`
* `cli` parses the given file (fails with proper message if the key is invalid)
* `cli` asks for the path for the sidechain signing key, proposing default from `sidechain.skey`
* `cli` outputs the final command - it has signatures included, no keys are present - and informs
  user that they should run it on a machine with cardano-node running.

### `register-3` wizard

This part executes the registration command.

* `cli` checks for if `chain config` is present and valid, if not it informs user that they should
  obtain it from the governance authority or run `prepare-configuration` command first, and exits.
* `cli` checks if `resources config` field `cardano_cli` is present, otherwise it asks user for it (
  like `prepare-configuration`) - since part 1 was run, it should be present
* `cli` checks if `resources config` field `cardano_node_socket_path` is present, otherwise it asks
  user for it (like `prepare-configuration`) - since part 1 was run, it should be present
* `cli` notifies user that payment signing key is used to sign the registration transaction and that
  the key will not be stored nor communicated over the network
* `cli` asks for the `cardano_payment_payment_signing_key_file` path
* `cli` executes `trustless-sidechain-cli register` internally with all the required parameters
* `cli` asks user if it should display the registration status (`Y/n`), if `Y`:
	* `cli` notifies user that it will query db-sync postgres state
	* `cli` uses substrate-node command (TBD) to query the user registration status
	* `cli` outputs the registration status (for epoch that is two epoch ahead of the current one)

## Configuration files examples

### chain config partner-chains-cli-chain-config.json

```json
{
	"bootnodes": [
		"/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp"
	],
	"cardano": {
		"network": 1,
		"security_parameter": 432,
		"active_slots_coeff": 0.05,
		"first_epoch_number": 5,
		"first_slot_number": 42000,
		"epoch_duration_millis": 43200,
		"first_epoch_timestamp_millis": 1590000000000
	},
	"chain_parameters": {
		"chain_id": 0,
		"governance_authority": "00000000000000000000000000000000000000000000000000000000",
		"genesis_utxo": "0000000000000000000000000000000000000000000000000000000000000000#0",
		"threshold_numerator": 2,
		"threshold_denominator": 3
	},
	"main_chain_addresses": {
		"committee_candidates_address": "addr_test1wz5qc7fk2pat0058w4zwvkw35ytptej3nuc3je2kgtan5dq3rt4sc",
		"d_parameter_policy_id": "a8629ed63b21472af8b18382303a2367b4707e3c2bc852f303a4612b",
		"permissioned_candidates_policy_id": "ee858c5273c62be11c83f4ff23ba435b35a53d3f92964055fb040849"
	}
}
```

### resources config partner-chains-cli-resources-config.json

```json
{
	"cardano_cli": "docker exec cardano-cli",
	"cardano_node_socket_path": "node.socket",
	"cardano_payment_verification_key_file": "payment.vkey",
	"db_sync_postgres_connection_string": "postgres://postgres_user:postgres_password@localhost/cexplorer",
	"substrate_node_base_path": "./data",
	"kupo": {
		"protocol": "http",
		"hostname": "localhost",
		"port": 1442
	},
	"ogmios": {
		"protocol": "http",
		"hostname": "localhost",
		"port": 1337
	}
}
```
