# Partner Chains Node

A [Substrate](https://substrate.io/) based blockchain node written in Rust, designed for operation of Cardano Partner chains. This node facilitates the creation and management of sidechains that integrate seamlessly with the Cardano ecosystem.

## Getting Started

These guides are designed to help you set and run your Partner chains node. Guides are available for three different types of users:

1. [Chain Builder](./docs/user-guides/chain-builder.md)
2. [Permissioned Validator](./docs/user-guides/permissioned.md)
3. [Registered Validator](./docs/user-guides/registered.md)

---
**NOTE**
The guides above are currently aimed at preview testnet only.

---

### Build

Build the Partner Chains Node from source with the following command:
```
cargo build --profile=production
```

### Downloads

Please see the [releases page](https://github.com/input-output-hk/partner-chains/releases) for the latest downloadable binaries and Docker images of the `partner-chains-node` and `partner-chains-cli`.

### Docker Image

The Docker image for the Partner chains node is available in the Github Container Registry:
```
docker pull ghcr.io/input-output-hk/partner-chains/partner-chains-node:latest
```

### Running partner chain nodes

To run the image:
```
docker run -v .run/data:/data:rw ghcr.io/input-output-hk/partner-chains/partner-chains-node:latest
```

The following commands will start a cluster of 5 nodes using preconfigured keys(chain_spec.rs):
```
docker run --env-file docker/.node-env --name=alice \
  -v .run/data:/data:rw -v .run:/ipc \
  -p 30333:30333 -p 9946:9946 -p 9934:9934 -u $UID:$GID \
  ghcr.io/input-output-hk/partner-chains/partner-chains-node:latest \
  --alice --base-path /data/alice --chain local --validator \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --state-pruning archive --blocks-pruning archive
docker run --env-file docker/.node-env --name=bob \
  -v .run/data:/data:rw -v .run:/ipc \
  -p 30334:30333 -p 9947:9946 -p 9935:9934 -u $UID:$GID \
  ghcr.io/input-output-hk/partner-chains/partner-chains-node:latest \
  --bob --base-path /data/bob --chain local --validator \
  --node-key 0000000000000000000000000000000000000000000000000000000000000002 \
  --state-pruning archive --blocks-pruning archive
docker run --env-file docker/.node-env --name=charlie \
  -v .run/data:/data:rw -v .run:/ipc \
  -p 30335:30333 -p 9948:9946 -p 9936:9934 -u $UID:$GID \
  ghcr.io/input-output-hk/partner-chains/partner-chains-node:latest \
  --charlie --base-path /data/charlie --chain local --validator \
  --node-key 0000000000000000000000000000000000000000000000000000000000000003 \
  --state-pruning archive --blocks-pruning archive
docker run --env-file docker/.node-env --name=dave \
  -v .run/data:/data:rw -v .run:/ipc \
  -p 30336:30333 -p 9949:9946 -p 9937:9934 -u $UID:$GID \
  ghcr.io/input-output-hk/partner-chains/partner-chains-node:latest \
  --dave --base-path /data/dave --chain local --validator \
  --node-key 0000000000000000000000000000000000000000000000000000000000000004 \
  --state-pruning archive --blocks-pruning archive
docker run --env-file docker/.node-env --name=eve \
  -v .run/data:/data:rw -v .run:/ipc \
  -p 30337:30333 -p 9950:9946 -p 9938:9934 -u $UID:$GID \
  ghcr.io/input-output-hk/partner-chains/partner-chains-node:latest \
  --eve --base-path /data/eve --chain local --validator \
  --node-key 0000000000000000000000000000000000000000000000000000000000000005 \
  --state-pruning archive --blocks-pruning archive
```
