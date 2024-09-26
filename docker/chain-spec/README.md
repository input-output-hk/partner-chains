## What is it?

This documentation describes a Docker image designed to build the partner-chains-node and generate a chain spec for a specified chain.

## Why Use Docker Instead of "cargo build"?

The runtime WebAssembly (WASM) binary compiled by default may not be deterministically reproducible. This means the compiler could produce slightly different WebAssembly bytecode with each build, which poses a problem for blockchain networks where all nodes must operate on the identical raw chain specification file.

Utilizing a Docker image for the build process ensures a consistent build environment and deterministic WASM builds.

## Do I need to run it everytime?

It is not necessary to use this Docker image for every build. For local development and testing, using the hardcoded chain spec is sufficient. However, a deterministic chain spec becomes crucial when initiating a new partner chain:

- A chain starter specifies all parameters for their chain.
- They generate a raw chain spec using this Docker image.
- The generated chain spec is distributed to the chain validators.

Validators have the option to either use the provided raw chain spec as is, trusting the chain starter, or regenerate the chain spec by building it locally with the same Docker image.

## Building the image

```
docker build . -t chain-spec
```
in the `docker/chain-spec` folder.

## Using the image

Before running the command make sure to give your Docker instance enough RAM to handle the linking stage.

To use the image, adjust the following template command according to your needs:

```
cd /path/to/partner-chains-node

docker run -it \
  -v ${PWD}:/build \
  -v ${PWD}/target/chain-spec:/build/target \
  -v /tmp/cargo-home:/cargo-home \
  -e CARGO_HOME=/cargo-home \
  -e CHAIN=staging \
  -e CHAIN_ID=12345 \
  -e GOVERNANCE_AUTHORITY=00000000000000000000000000000000000000000000000000000000 \
  -e SPEC_FILE_UID=$(id -u) \
  -e SPEC_FILE_GID=$(id -g) \
  chain-spec:latest
```

#### Line by line breakdown:

Start the container. `-it` can be used optionally, to stay in the container after the build finishes.
```
docker run -it \
```

Mount the repo to `/build`. One of the conditions for deterministic builds is a set path.
```
-v /path/to/partner-chains-node:/build \
```

Mount the `target` folder for incremental builds. It can be `/tmp/target` if you don't want touch
your local build files.
```
-v /path/to/partner-chains-node/target/chain-spec:/build/target \
```

Cache cargo home for faster builds.
```
-v /tmp/cargo-home:/cargo-home \
-e CARGO_HOME=/cargo-home \
```

Set an env variable to specify which chain to use. Currently, we only have "local" (devnet) and "staging". These IDs are used in `build-spec` command and pattern matched in `command.rs`, `SubstrateCli impl` `load_spec` function.
Omit, if `--chain` option should not be used for the build-spec command.
```
-e CHAIN=staging \
```

Set the genesis utxo and governance authority chain parameters.
See entrypoint.sh file for legacy parameters defaults.
```
-e GENESIS_COMMITTEE_UTXO="0000000000000000000000000000000000000000000000000000000000000000#0" \
-e GOVERNANCE_AUTHORITY=00000000000000000000000000000000000000000000000000000000 \
```

Build chain spec in raw format, it's optional, omit if you want to build without `--raw` flag.
```
-e RAW=true \
```

Change the owner of the generated chain spec file to the current user.
Omit if you don't want to change the owner, generated file ownership will depend on the docker.
```
-e SPEC_FILE_UID=$(id -u) \
```

Change the group ownership of the generated chain spec file to the current user group.
Omit if you don't want to change the group, generated file group ownership will depend on the docker.
```
-e SPEC_FILE_GID=$(id -g) \
```

Image to use
```
chain-spec:latest
```

---
When the build finishes (it might take a while) you should find the generated chain spec in the repo root folder named `chain-spec.json`.

Now you can use this chain spec in your node by providing it in the `--chain` argument:
```
./target/debug/partner-chains-node \
  --base-path /tmp/alice \
  --chain chain-spec.json \
  --alice \
  --port 30333 \
  --rpc-port 9933 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url "wss://telemetry.polkadot.io/submit/ 0" \
```
