# Partner Chains Node

A [Substrate](https://substrate.io/)-based blockchain node written in Rust, designed for operation of Cardano Partner Chains, this node facilitates the creation and management of sidechains that integrate seamlessly with the Cardano ecosystem.

This alpha release is just the beginning of the journey. It is intended to gather early feedback from the community and is provided "as is." It should not be used in live production networks. Use at your own risk. We welcome and appreciate your feedback!

## Getting Started

These guides are designed to help you set and run your Partner Chains node. Guides are available for three different types of users:

1. [Chain Builder](./docs/user-guides/chain-builder.md)
2. [Permissioned Validator](./docs/user-guides/permissioned.md)
3. [Registered Validator](./docs/user-guides/registered.md)

---
**NOTE**

The guides above are currently aimed at preview testnet only.

---

### Build

Build the Partner Chains node from source with the following command:
```
cargo build --profile=production
```

### Downloads

Please see the [releases page](https://github.com/input-output-hk/partner-chains/releases) for the latest downloadable binaries and Docker images of the `partner-chains-node` and `partner-chains-cli`.

### Docker Image

The Docker image for the Partner Chains node is available in the Github Container Registry:
```
docker pull ghcr.io/input-output-hk/partner-chains/partner-chains-node:latest
```

## How to Use

Refer to the [documentation](docs/user-guides) for detailed instructions on how to use the node and CLI tools.
.
