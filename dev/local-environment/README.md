# Partner Chains Local Environment

This stack is designed to run a 5 x Partner Chains node local environment for a partner chain. It is based on the custom IO Substrate image.

![alt text](pc-local-env.png)

The local environment includes:

- 5 x Partner Chains Nodes (3 x permissioned, 2 x registered)
- 3 x Cardano Nodes running private testnet with pre-configured genesis files (2 minutes epochs)
- 1 x PostgreSQL database
- 1 x Db-sync
- 1 x Ogmios
- 1 x Kupo
- 1 x Ubuntu / NodeJS image for running pc-contracts-cli

The stack `setup.sh` script will create a docker-compose.yml stack configuration files, and populate an .env file with environment values. The stack can be deployed with `docker-compose up -d`. 

## Local env - step by step

- When first run, all images will be pulled from public repositories. This stage may take some time. The stack will then be built and run.
- When the stack is running, the 3 Cardano nodes will peer and being block production. This is a private testnet and will not connect to the public Cardano network, but rather from a pre-configured genesis file. 
- Once the Cardano chain is synced, Ogmios, Kupo and DB-Sync will in turn connect to the Cardano node node.socket and begin syncing the chain. 
- The pc-contracts-cli will insert D parameter values and register Partner Chains Node keys with the Cardano chain.
- Once Postgres is populated with the required data, the Partner Chains nodes will begin syncing the chain and will begin block production after 2 main chain epochs.

## Configuring the environment

Run `setup.sh` script to create the environment .env values and docker-compose.yml

```
chmod +x setup.sh
bash setup.sh`
```

The `setup.sh` script also support a `--non-interactive` flag to accept default configuration settings, and some additional arguments to set configuration values directly. See the help page for more information

```
bash setup.sh --help
```

## Starting the environment

Once initialized, deploy the local environment from .env values with the following:

```
docker compose up -d
```

We recommend using a visual Docker UI tool such as [lazydocker](https://github.com/jesseduffield/lazydocker) or [Docker Desktop](https://www.docker.com/products/docker-desktop/) for following the live logs and performance of all containers in the environment. Each component has been scripted to provide verbose logging of all configuration actions it is performing to demonstrate the end-to-end setup of a Cardano Partner Chain.

## Stopping the environment

When stopping the stack, it is mandatory to also wipe all volumes. The environment does not yet support persistent state. To tear down the environment and remove all volumes, use the following: 

```
docker compose down --volumes
```

## Using custom node image

To use custom node image one simply has to update `PARTNER_CHAINS_NODE_IMAGE` (docker image) and `PARTNER_CHAINS_NODE_URL` (node artifact, used to build chain-spec and generate signatures for SPO registration) in `setup.sh` script.

Also, make sure that `PC_CONTRACTS_CLI_ZIP_URL` version is compatible with your custom node.
