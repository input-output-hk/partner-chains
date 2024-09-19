# Partner Chain Dependencies

For a local cluster of nodes we need to have the dependencies running.
The main chain follower reads from **postgres** that **db-sync** writes to.
**db-sync** itself requires UDS connection to **cardano-node**.

For executing **partner-chains-smart-contracts** CLI commands **ogmios** and **kupo** are required.
Both of them open HTTP ports that `pc-contracts-cli` is looking for by default (1337 and 1442).

This document will help you with spinning up the dependencies running in either docker; or
_process-compose_; the latter utilizing containerless, natively built binaries via nix.

## Using docker

This section uses **cardano-node**, **cardano-db-sync** from ghcr.io repository,
and **postgres**, **kupo**, and **ogmios** from docker.io repository.
Setup uses docker volumes for cardano-node data, postgres data and substrate node data, and
**cardano-node** Unix domain socket for **cardano-db-sync**.

### Docker-compose

The set-up of dependencies is maintained in the docker compose file.
If you opt to run images individually, please use this file as reference for images configuration.

To start the dependencies, from the root directory of this project, run:
```
docker-compose -f docker/sidechain-dependencies/docker-compose.yml up -d
```

### Docker settings for Apple Silicon

- General
  - Use Virtualization framework
  - VirtioFS file sharing system
- Resources
  - CPUs: 5
  - Memory: 12 GB
- Features in development
  - Use Rosetta for x86/amd64 emulation on Apple Silicon

## Using the containerless nix stack

To run the dependency stack without requiring OCI images we have an alternate
implementation that utilizes
[process-compose](https://github.com/F1bonacc1/process-compose) and nix shells.

Benefits:

- Access to the ECR not required; no images/containers are used to run the stack
- Thus, no need to build/load/share images
- Native local builds for each supported architecture:
  - x86_64-linux
  - x86_64-darwin
  - arm64-darwin
- Works as expected while connecting/disconnecting with the VPN

### Setup and run
To enter the shell with this stack setup using direnv, add an .envrc.local file with contents:
```sh
export NIX_SHELL=".#process-compose"
```

Then run:
```sh
direnv allow
```

You will get a new menu command: `sidechains-stack`. The command is a wrapper to
the normal `process-compose` command, so check the `--help` for additional
command line options (such as `--tui=false`).

The data for the stack is output locally to the `.run` directory from where the
command is launched.  This may change or be adjusted in the future to match with
the data in docker-compose.  In addition, be aware that you should launch it in
the same project (root) directory or it will create a new `.run` from your
current `pwd`.

### Other notes and features

Process-compose runs an http server with swagger, and by default in this setup
runs it on port `8081`.  You can attach to remote process-compose stacks with:
```sh
sidechains-stack attach --address <ip(default localhost)> --port 8081
```
You can also access the swagger UI through the browser e.g. by visiting
`127.0.0.1:8081`
