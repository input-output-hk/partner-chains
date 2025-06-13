# Dynamic Local Test Environment

This document outlines the setup and operation of the dynamic local test environment for Partner Chains. This environment is designed for flexibility, allowing developers to easily configure and launch a large-scale network of Partner Chains nodes with automated setup and registration.

Unlike the previous static `local-environment`, which used a fixed number of validators with pre-generated keys, this dynamic environment automates the entire lifecycle, including key generation, funding, and on-chain registration.

## Key Features

- **Dynamic Configuration**: Easily set the number of "permissioned" and "registered" validator nodes to launch.
- **Automated Lifecycle Management**: Handles the entire process of key generation, Cardano address funding, and SPO registration for all validators.
- **Dynamic Node Discovery**: Nodes use dynamic node-keys, PeerIDs, and public addresses for automatic peer discovery within the Substrate network.
- **Comprehensive Stack**: Includes all necessary components for a fully functional test environment: Cardano node, DB-Sync, Ogmios, and the Partner Chains nodes.

## System Requirements

Running the local environment requires a machine with adequate resources. 

## Configuration

### Node Count

At the top of the `setup.sh` script, you can configure the size of the network:

```sh
NUM_PERMISSIONED_NODES_TO_PROCESS=10
NUM_REGISTERED_NODES_TO_PROCESS=10
```

- `NUM_PERMISSIONED_NODES_TO_PROCESS`: Defines the number of initial, permissioned nodes.
- `NUM_REGISTERED_NODES_TO_PROCESS`: Defines the number of additional "registered" nodes that will be spun up and registered as SPOs.

### Custom Node Image

To use a custom Partner Chains node image, simply update the `PARTNER_CHAINS_NODE_IMAGE` variable at the top of the `setup.sh` script.

## Automated Setup Process Explained

The automation is handled by a series of scripts that execute in a specific order.

### 1. `setup.sh`

This is the main orchestration script. When you run `bash setup.sh`, it performs the following steps:

1.  **System Checks**: Detects the OS and prompts for configuration choices (ports, resource limits, etc.) in interactive mode.
2.  **Configuration Generation**:
    -   Creates a `.env` file with all the environment variables for Docker Compose.
    -   Generates individual entrypoint scripts for each Partner Chains node inside `configurations/partner-chains-nodes/`. These entrypoints include a staggered start delay (`sleep`) to prevent all nodes from starting at once.
3.  **Docker Compose Manifest Generation**:
    -   Constructs the main `docker-compose.yml` file by adding service definitions for each Partner Chains node and appending the core Cardano stack (`cardano-node-1`, `postgres`, etc.) from the `modules/` directory.

A key feature of the generated configurations is the use of dynamic discovery flags for the Substrate node:

```bash
/usr/local/bin/partner-chains-node \\
  # ... other flags
  --node-key="$(openssl rand -hex 32)" \\
  --listen-addr=/ip4/0.0.0.0/tcp/30333 \\
  --public-addr="/dns4/partner-chains-node-[...]/tcp/30333/p2p/\$PEER_ID" &
```

- A unique `--node-key` is generated for each node on startup, from which a `PEER_ID` is derived. The node then advertises its public, DNS-based address for discovery.

### 2. `cardano/entrypoint.sh` (inside `cardano-node-1`)

This script bootstraps the Cardano side of the environment.

1.  **Starts Cardano Node**: It waits for configuration files from the setup container, then starts the `cardano-node`.
2.  **SPO Registration**: It runs a loop that watches the `/shared/spo-certs/` directory. When a new validator's `.cert` file appears, it submits it to the Cardano network, officially registering the validator as an SPO.

### 3. `partner-chains-setup/entrypoint.sh` (inside `partner-chains-setup`)

This is the core script for automating the validator lifecycle.

1.  **Setup**: It waits for the Cardano node and Ogmios, then generates the dynamic Cardano genesis files.
2.  **Key Generation**: It programmatically generates all necessary keys (payment, stake, cold, VRF, keystore) for every configured validator.
3.  **Funding**: It builds and submits Cardano transactions to fund the payment addresses of all validators from a genesis UTXO.
4.  **Certificate Generation**: For each validator, it generates the necessary stake address and stake pool registration certificates, placing the final certificate in `/shared/spo-certs/` to be picked up by the Cardano node.
5.  **Ready Signal**: Once complete, it creates a `partner-chains-setup.ready` file, signaling the Partner Chains nodes that they can start.

## Usage

### 1. Initialise the Environment

Run `setup.sh` to enter the setup wizard for initialising the environment (`.env` file) and `docker-compose.yml`.

```bash
chmod +x setup.sh
bash setup.sh
```

Alternatively, run in non-interactive mode to accept default settings:
```bash
bash setup.sh --non-interactive
```

### 2. Start the Environment

Once initialized, deploy the local environment with:

```bash
docker compose up -d
```

### 3. Monitor the Environment

We recommend using a visual Docker UI tool such as [lazydocker](https://github.com/jesseduffield/lazydocker) or Docker Desktop for following the live logs and container performance.

You can also monitor logs directly from the command line:
-   **Cardano SPO registrations**: `docker logs cardano-node-1 -f | grep -E "DEBUG|LOG|WARN"`
-   **Full stack setup progress**: `docker logs partner-chains-setup -f`
-   **Partner Chains node startup**: `docker logs partner-chains-node-permissioned-1 -f` (or any other node).

### 4. Stop the Environment

To tear down the environment and remove all data, it is mandatory to also wipe all volumes.

```bash
docker compose down --volumes
```

## Advanced Usage & Customization

The `setup.sh` script provides several flags for more advanced configurations. The help dialogue will always show the latest available features.

```bash
$ bash setup.sh --help
Usage: ./setup.sh [OPTION]...
Initialize and configure the Docker environment.
  -n, --non-interactive     Run with no interactive prompts and accept sensible default configuration settings.
  -d, --deployment-option   Specify one of the custom deployment options (1, 2, 3, or 4).
  -p, --postgres-password   Set a specific password for PostgreSQL (overrides automatic generation).
  -i, --node-image          Specify a custom Partner Chains Node image.
  -t, --tests               Include tests container.
  -h, --help                Display this help dialogue and exit.
```
