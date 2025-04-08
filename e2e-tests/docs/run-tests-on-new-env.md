# How to run system tests for partner chains on a new environment

## Prerequisites

* A partner chain node with JSON-RPC API available
* A node with Postgres SQL (for automated test data)
* A node with [cardano-node](https://github.com/IntersectMBO/cardano-node) + [cardano-db-sync](https://github.com/IntersectMBO/cardano-db-sync) + [postgres](https://www.postgresql.org/) (running locally OR in a Kubernetes pod)
* [ogmios](https://github.com/CardanoSolutions/ogmios) (running locally OR in a Kubernetes pod)
* [partner-chains-node](https://github.com/input-output-hk/partner-chains) (running locally OR in a Kubernetes pod)
* cardano-cli (from a local cardano-node OR in a Kubernetes pod)

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

- `host` - hostname or IP address of the node
- `port` - port number of the node
- `rpc_url` - URL of the JSON-RPC API
- `ws_url` - URL of the WebSocket API
- `p2p_url` - URL of the P2P API
- `metrics_url` - URL of the metrics API
- `keys` - path to the node keys

### 4. Add `<env>_stack.json` to `config/<blockchain>/<env>` folder

`<env>_stack.json` configuration file represents the tools and services required for test execution.

#### `<env>_stack.json` structure:

- `ogmios_host` - hostname or IP address of the ogmios service
- `ogmios_port` - port number of the ogmios service
- `validator_name` - name of the validator pod to use (e.g., "ci-preview-validator-1", "staging-preview-validator-1", "alice")
- `tools` - configuration for the tools required for test execution
  - `cardano_cli` - configuration for the cardano-cli tool
  - `partner_chains_node` - configuration for the partner-chains-node tool
  - `bech32` - configuration for the bech32 tool

#### Tools configuration:

Each tool can be configured to run:
- locally on the test runner machine
- in a Kubernetes pod using kubectl exec

#### `<env>_stack.json` template:

```
{
    "stack_config": {
        "ogmios_host": <STRING>,
        "ogmios_port": 1337,
        "validator_name": <STRING>, // e.g., "ci-preview-validator-1", "staging-preview-validator-1", "alice"
        "tools": {
            "cardano_cli": {
                "cli": "cardano-cli",
                "shell": "kubectl exec -it ${stack_config[validator_name]} -c cardano-node -n <namespace> --"
            },
            "partner_chains_node": {
                "cli": "/usr/local/bin/partner-chains-node",
                "shell": "kubectl exec -it ${stack_config[validator_name]} -c substrate-node -n <namespace> --"
            },
            "bech32": {
                "cli": "bech32",
                "shell": "kubectl exec -it ${stack_config[validator_name]} -c substrate-node -n <namespace> --"
            }
        }
    }
}
```

Where `<namespace>` is the Kubernetes namespace where your pods are running (e.g., "sc", "staging-preview", "ci-preview").

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
$ pytest -rP -v --blockchain <blockchain> --env <env> --log-cli-level debug -vv -s -m "not probability"
```
where:

* `--env` - target node environment
* `--blockchain` - target type of blockchain: substrate, `<blockchain>`
* `--log-cli-level` - log level for output (info, debug, warning, critical, error)
* `-m` - pytest markers to filter tests for execution
* `-vv` - pytest parameter to show duration of tests
* `-s` - pytest parameter to show test output to console
* `-rP` - pytest parameter to show skipped tests
