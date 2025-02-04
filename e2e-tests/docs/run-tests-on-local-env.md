# How to run system  tests on a partner-chains local environment

## Prerequisites

- Docker Desktop, lazydocker
- python3, pip

## Steps 

1. Configure partner-chains local environment by running [setup.sh](https://github.com/input-output-hk/partner-chains/blob/master/dev/local-environment/setup.sh)
   * If you want to use a pre-configured postgres password from /secrets/local/local.json: `$ ./setup.sh -p <PASSWORD>`
   * If you want to use generated password from local env: run `$ ./setup.sh -n`. Password is saved in `.env` file, you will need it later
2. Run local environment: `$ docker-compose up -d` and wait until the partner chains nodes finish syncing
3. Get initial_timestamp value: `$ docker exec cardano-node-1 cat /shared/cardano.start`
4. Set postgres passwords
   * If you used a pre-configured password, skip this step
   * If you used a generated password - update db and dbSync password values in `secrets/substrate/local/local.json` with the POSTGRES_PASSWORD env variable value from the `.env` file
5. Update `main_chain.init_timestamp` at config/substrate/local_nodes.json to the resulting value of `docker exec cardano-node-1 cat /shared/cardano.start`
6. Create and activate virtual environment
```bash
   $ pip install virtualenv
   $ python -m venv venv
   $ source venv/bin/active
```
1. Install project dependencies: `$ pip install -r requirements.txt`
2. Run partner-chains tests on partner-chains local environment
```bash
$ pytest -rP -v --blockchain substrate --env local --stack local --log-cli-level debug -vv -s -m "not active_flow and not passive_flow and not probability"
```

## Substrate Portal

After you start the node locally, you can interact with it using the hosted version of the [Polkadot/Substrate Portal](https://polkadot.js.org/apps/#/explorer?rpc=ws://localhost:9945) front-end by connecting to the local node endpoint.