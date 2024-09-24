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

## Block Production Rewards

The Partner Chains node provides a framework for block production and reward calculation, but the actual reward distribution is implemented by each Partner Chain instance. 
The process described below allows for a flexible and fair distribution of rewards, taking into account both block production and stake delegation, while ensuring that only registered participants receive rewards.

| Component | Description |
|-----------|-------------|
| Registration | Validators and delegators must register their Cardano stake keys with Partner Chain addresses |
| Block logging | Partner Chain nodes log block production data, including beneficiaries |
| Reward calculation | Determined by the Partner Chain instance (e.g., N tokens per block) |
| Distribution | Typically split between block producers and delegators based on stake |
| Smart contracts | Track permissioned candidates, registered candidates, and registered delegators |

### For Partner Chains Node Operators

1. Implement a reward distribution system using the block production data provided by the Partner Chains node. 
2. Set up and manage the required smart contracts on Cardano.
3. Automate the reward calculation and distribution process within your consensus layer.
4. Establish a registration process for your validators and delegators.
5. Determine your reward split between block producers and delegators.

### For Stake Pool Operators (SPOs)

- Register to become a Partner Chain validator.
- Encourage your delegators to register for Partner Chain rewards.
- Monitor block production and reward distribution for your pool.

### More Details

1. Registration Process: Stake Pool Operators (SPOs) who want to become Partner Chain validators must register. They then inform their delegators to register if they want to receive rewards. Delegators must associate their Cardano stake key with a Partner Chain address through a registration process, which could involve using a CLI to generate keys and sign a transaction.
2. Block Production Tracking: The Partner Chain node logs block production data. Each block has a beneficiary, identified by a SizedByteString (such as `SizedByteStrings(0x1)`, `SizedByteStrings(0x2)`, etc.). At the end of each Partner Chain epoch, the node summarizes who produced how many blocks.
3. Reward Calculation: The Partner Chain instance sets a reward amount per block. For example, if 500 tokens are rewarded per block and 20 blocks were produced in an epoch, the total reward would be 10,000 tokens. This is typically split between block producers (for example, 10% or 1,000 tokens) and delegators (for example, 90% or 9,000 tokens).
4. Stake-based Distribution: The delegator portion of the rewards is distributed based on the proportion of registered stake. For example, if there's 1,500,000 ADA of registered stake, and a delegator has staked 15,000 ADA, they would receive (15,000 / 1,500,000) * 9,000 = 90 tokens.
5. Smart Contract Implementation: Three main smart contracts are involved: 
   a) Permissioned candidates contract: Lists approved validators with their public keys.
   b) Registered candidates contract: Lists registered SPOs who can act as validators.
   c) Registered delegators contract: Associates Partner Chain addresses with Cardano stake keys and shows which pool each address is delegated to.
6. Reward Distribution Mechanism: The Partner Chain instance uses the block production data from the node logs and the information from the smart contracts to determine the reward distribution. It then implements the transactions to send the appropriate amount of rewards to each validator and registered delegator.
7. Automation and Verification: The entire process needs to be automated within the consensus layer of the Partner Chain. Automated tests will be implemented to verify that the reward distribution is functioning correctly over time.
