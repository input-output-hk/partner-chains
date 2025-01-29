# How to run system tests for partner chains on a new environment

## Prerequisites

* A partner chain node with JSON-RPC API available
* A node with Postgres SQL (for automated test data)
* A node with [cardano-node](https://github.com/IntersectMBO/cardano-node) + [cardano-db-sync](https://github.com/IntersectMBO/cardano-db-sync) + [postgres](https://www.postgresql.org/) (running locally OR on one of the partner-chains nodes)
* [ogmios](https://github.com/CardanoSolutions/ogmios) and [kupo](https://github.com/CardanoSolutions/kupo) (running locally OR on the remote host)
* [partner-chains-node](https://github.com/input-output-hk/partner-chains) (running locally OR on the remote tools host)
* cardano-cli (from a local cardano-node OR on the remote tools host)

**NOTE:**

- `<env>` is a placeholder for a your environment name
- `<node>` is a placeholder for partner chain node name
- `<blockchain>` is a placeholder for custom blockchain
- If your environment is for Substrate-based partner-chain, you can add your configuration files under `config/substrate` folder
- If you want to support totally different custom blockchain - create a separate `config/<blockchain>` and `secrets/<blockchain>` folders for configuration files and keys

## Steps

### 1. Add keys for each partner chain node

Copy the following keys to `secrets/<blockchain>/<env>/keys/<node>`:

- cold.skey
- cold.vkey
- payment.skey
- sidechain.skey (ECDSA partner chain key)

### 2. Add governance authority key

Add payment signing key of the governance authority as `init.skey` to `secrets/<blockchain>/<env>/keys/governance_authority`

### 3. Add `<env>_nodes.json` to `config/<blockchain>/<env>` folder

`<env>_nodes.json` configuration file represents basic partner chain configuration.

#### `<env>_nodes.json` structure:

- `deployment_mc_epoch` - mainchain epoch when partner chain was deployed
- `initial_pc_epoch` - first partner chain epoch number
- `genesis_utxo` - genesis utxo from `sidechain_getParams()`
- `deployment_version` - release version of partner chains
- `test_environment` - name of your environment
- `nodes_config` - list of node configurations for partner chain
- `selected_node` - chosen partner chain node (key from config)
- `block_duration` - duration of one partner chain block
- `slot_in_epoch` - amount of slots in one partner chain epoch
- `committee_participation_tolerance` - maximum tolerance percentage for committee participants

#### Each partner chain node is expected to have:

- `name` - name of partner-chain `<node>`
  - `host` - ip address of partner chain node
  - `port` - port of partner chain node
  - `aura_ss58_address` - SS58 address for Substrate Sr25519 key
  - `public_key` - ECDSA public key (hex) for partner chain
  - `aura_public_key` - Sr25519 public key (hex)
  - `grandpa_public_key` - Ed25519 public key (hex)
  - `permissioned_candidate` - indicator whether this node is permissioned candidate (true|false)
  - `block_reward_id` - reward id configured at the partner chain startup
  - `key_files` - a set of keys for registered candidates
    - `cardano_payment_key` - path to payment key of registered candidate (payment.skey from step 1)
    - `spo_signing_key` - path to signing key of registered candidate (cold.skey form step 1)
    - `spo_public_key` - path to public key of registered candidate (cold.vkey from step 1)
    - `sidechain_signing_key` - path to ECDSA private key (hex) (sidechain.skey from step 1)

Additionally, you can add configuration of the main chain to `<env>_nodes.json`.

E.g. for Cardano Preview it will be:

```json
    "main_chain": {
        "network": "--testnet-magic 2",
        "epoch_length": 86400,
        "slot_length": 1,
        "active_slots_coeff": 0.05,
        "security_param": 432,
        "init_timestamp": 1666656000,
        "block_stability_margin": 0
    }
```

#### `<env>_nodes.json` template:

```
{
    "deployment_mc_epoch": <INT>,
    "genesis_utxo": <STRING>,
    "committee_participation_tolerance": <INT>,
    "main_chain": {
        "network": <STRING>,
        "epoch_length": <INT>,
        "slot_length": <INT>,
        "active_slots_coeff": <FLOAT>,
        "security_param": <INT>,
        "init_timestamp": <INT>,
        "block_stability_margin": <INT>
    },
    "nodes_config": {
        "nodes": {
            "<node>": {
                "host": <STRING>,
                "port": <INT>,
                "aura_ss58_address": <STRING>,
                "public_key": <STRING>,
                "aura_public_key": <STRING>,
                "grandpa_public_key": <STRING>",
                "permissioned_candidate": <BOOLEAN>,
                "block_rewards_id": <STRING>
                "key_files": {
                    "cardano_payment_key": <STRING>,
                    "spo_signing_key": <STRING>,
                    "spo_public_key": <STRING>,
                    "sidechain_signing_key": <STRING>
                }
            },
            ...
        },
        "governance_authority": {
            "mainchain_address": <STRING>,
            "mainchain_key": "./secrets/<blockchain>/<env>/keys/<node>/init.skey"
        },
        "selected_node": <node>,
        "node": "${nodes_config[nodes][${nodes_config[selected_node]}]}",
        "token_conversion_rate": <INT>, // default - 9
        "block_duration": <INT>,
        "slots_in_epoch": <INT>,
        "token_policy_id": <STRING>,
        "fees": {
            "ECDSA": {
                "lock": <INT>,
                "send": <INT>
            },
            "SR25519": {
                "lock": <INT>,
                "send": <INT>
            }
        }
    },
    "block_encoding_suffix_grandpa": "3903",
    "block_encoding_suffix_aura": "8902"
}
```

### 4. Add `<env>_stack.json` to `config/<blockchain>/<env>` folder

`<env>_stack.json` configuration file represents connection strings to partner chain dependencies (ogmios, kupo) and binaries (partner-chains-node, sidechain-main-cli, cardano-cli).

**NOTE:**
- **ogmios** and **kupo** services can be executed on the remote host or made available on test runner machine
- partner chains binaries can be made available on the test runner machine or on the remote host
  - if you want to use the remote host - you need to configure your own SSH keys

#### `<env>_stack.json` template:

```
{
    "stack_config": {
        "ogmios_host": <STRING>,
        "ogmios_port": 1337,
        "kupo_host": <STRING>,
        "kupo_port": 1442,
        "tools_shell": "/bin/bash",
        "tools_host": <STRING,
        "ssh": {
            "username": "root",
            "host": "${stack_config[tools_host]}",
            "port": 22,
            "host_keys_path": "config/<blockchain>/known_hosts",
            "private_key_path": <STRING>
        },
        "tools": {
            "cardano_cli": {
                "cli": <STRING>, // path to cardano-cli binary
            },
            "sidechain_main_cli": {
                "cli": <STRING> // path to sidechain-main-cli binary
            },
            "partner_chains_node": {
                "cli": <STRING> // path to partner-chains-node binary
            },
            "bech32": {
                "cli": <STRING> // path to bech32 binary
            }
        }
    }
}
```

### 5. Add `<env>.json` to `secrets/<blockchain>`

`<env>.json` configuration file represents connection details of databases used within tests (`cexplorer` - for cardano-db-sync, `qa_db` - for automated test data)

#### `<env>.json` template:

```
{
	"db": {
		"type": "postgresql",
		"username": "postgres",
		"password": <STRING>,
		"host": <STRING>,
		"port": "5432",
		"name": "qa_db",
		"url": "${db.type}://${db.username}:${db.password}@${db.host}:${db.port}/${db.name}"
	},
	"dbSync": {
		"type": "postgresql",
		"username": "postgres",
		"password": <STRING>,
		"host": <STRING>,
		"port": "5432",
		"name": "cexplorer",
		"url": "${dbSync.type}://${dbSync.username}:${dbSync.password}@${dbSync.host}:${dbSync.port}/${dbSync.name}"
	},
	"wallets": {
		"faucet-0": {
			"scheme": "ECDSA",
			"address": <STRING>,
			"secret_seed": <STRING>,
			"public_key": <STRING>
		}
	}
}
```

### 6. Run tests on your custom environment

```bash
$ pytest -rP -v --blockchain <blockchain> --env <env> --stack <env> --log-cli-level debug -vv -s -m "not active_flow and not passive_flow and not probability"
```
where:

* `--env` - target node environment
* `--stack` - target dependencies stack
* `--blockchain` - target type of blockchain: substrate, evm, `<blockchain>`
* `--log-cli-level` - log level for output (info, debug, warning, critical, error)
* `-m` - pytest markers to filter tests for execution
* `-vv` - pytest parameter to show duration of tests
* `-s` - pytest parameter to show test output to console
* `-rP` - pytest parameter to show skipped tests
