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

## Block Production Rewards for Validators

The Partner Chains node provides a simple mechanism for exposing the mapping of block beneficiaries with produced blocks. 
The chain builder is responsible for using this input to accurately calculate and distribute block rewards on their partner chain, following the tokenomics they will design and implement. 
For managing payouts, they will leverage the provided artifacts to distribute payments to validators on the partner chain ledger, applying their specific business logic.

| Component | Description |
|-----------|-------------|
| Registration | Validators register their Cardano cold secret key with their Partner Chain keys by producing two signatures and providing the public keys of each (Cardano and Partner Chains) so that others can verify that the signature matches the public key |
| Block logging | Partner Chain nodes log block production data, including beneficiaries |
| Reward calculation | Determined by the Partner Chain instance (for example, N tokens per block) |
| Smart contracts | Track permissioned candidates, and registered candidates |

### For Partner Chains Node Operators

1. Implement a reward distribution system using the block production data provided by the Partner Chains node. 
2. Set up and manage the required smart contracts on Cardano.
3. Automate the reward calculation and distribution process within the consensus layer.
4. Establish a registration process for validators.

### For Stake Pool Operators (SPOs)

- Register to become a partner chain validator.
- Monitor block production and reward distribution for your pool.

### More Details

1. Block Production Tracking: The Partner Chain node logs block production data. Each block has a beneficiary, identified by Partner Chain receiving addresses (such as `SizedByteStrings(0x1)`, `SizedByteStrings(0x2)`, etc.). At the end of each Partner Chain epoch, the node summarizes who produced how many blocks.
2. Smart Contract Implementation: Two main smart contracts are involved: 
   a. Permissioned candidates contract: Lists approved validators with their public keys.
   b. Registered candidates contract: Lists registered SPOs who can act as validators.
3. Reward Distribution Mechanism: The Partner Chain instance uses the block production data from the node logs and the information from the smart contracts to determine the reward distribution. It then implements the transactions to send the appropriate amount of rewards to each validator.
4. Automation and Verification: The entire process needs to be automated within the consensus layer of the Partner Chain. Automated tests will be implemented to verify that the reward distribution is functioning correctly over time.
