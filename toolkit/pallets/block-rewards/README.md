# Block Rewards Pallet

## Overview

The Block Rewards pallet serves as an accounting system for tracking and managing rewards generated through block production in a partner chain network. It implements a mechanism that tracks the accumulation of rewards by block beneficiaries over time, providing the underlying infrastructure necessary for fair reward distribution.

At its core, this pallet maintains an accounting ledger of who should receive rewards for block production and in what quantities. It operates on the principle that each successfully produced and finalized block generates a specific amount of rewards (determined by configurable reward point calculation strategies), which are credited to the designated beneficiary for that block.

Unlike most other pallets that implement complete functionality independently, the Block Rewards pallet is intentionally designed as part of a modular system with clear separation of concerns:

1. **Reward Accounting**: The Block Rewards pallet itself focuses solely on maintaining an accurate accounting of earned rewards, tracking who has earned what.

2. **Reward Processing**: The actual processing and distribution of rewards is left to chain-specific runtime code, providing maximum flexibility in how rewards are ultimately delivered.

3. **Beneficiary Identification**: A separate inherent data provider component supplies the beneficiary ID for each block, allowing different networks to implement custom approaches to determining block beneficiaries.

This modular design offers several advantages:

- **Customizability**: Partner chains can implement their own reward processing logic to meet their specific economic models and requirements.
- **Separation of Responsibilities**: The pallet maintains a clean separation between tracking reward entitlement and processing reward payments.
- **Flexible Processing Frequency**: Rewards can be processed at any desired frequency, not necessarily on every block, enabling efficient batched processing.

The pallet works closely with the inherent data system to automatically receive beneficiary information for each block, ensuring that reward accounting happens seamlessly as part of normal block processing. This approach enables automatic reward tracking without requiring explicit extrinsic calls for each block.

## Purpose

This crate implements the runtime component of the block rewards whose task is to track allocation of rewards for block production and allow processing of this information for reward payouts.

This crate works together with the `sp-block-rewards` crate to implement this functionality.

The pallet serves several specific purposes:

- Tracks who is entitled to receive rewards for producing blocks
- Accumulates reward points over time until they are processed
- Provides a clean mechanism to retrieve and reset reward data when processing occurs
- Ensures rewards are only credited for blocks that are finalized
- Supports customizable reward calculation strategies

## Primitives

The Block Rewards pallet relies on primitives defined in the `toolkit/primitives/block-rewards` crate.

## Usage

### Block beneficiary

Rewards for block production are credited to a `beneficiary`, identified by a `T::BeneficiaryId`, which is a chain-specific type that needs to be provided by the developer.
When each block is produced, the `beneficiary` is determined by the inherent data provided by the `BlockBeneficiaryInherentProvider`. This inherent data provider can be construed in an arbitrary way, but `BlockBeneficiaryInherentProvider::from_env` helper method on the type is provided to fetch the ID from the node's local environment.

The value of the beneficiary ID is chosen at the sole discretion of each particular block producing node's operator and is not subject to any checks beyond simple format validation (in particular, it is not checked whether the node operator controls any keys related to the ID).

### Block value

Rewards credited for each block's production can be calculated arbitrarily, by providing the `T::BlockRewardPoints` type representing the rewards and `T::GetBlockRewardPoints: GetBlockRewardPoints` type implementing the logic determining current block value.

For simple cases, type `SimpleBlockCount: GetBlockRewardPoints` is provided that assigns a constant value of 1 to every block, for any `T::BlockRewardPoints` implementing the `One` trait.

### Pallet operation

The pallet ingests the inherent data and keeps a tally of beneficiaries and their accumulated rewards in its on-chain storage. The rewards are only credited for blocks whose production was successfuly finalized.

### Processing the rewards

To make the reward information available for processing, the pallet exposes the `get_rewards_and_clear` function, which returns all accumulated reward data and resets the pallet storage. This function should be called from the chain-specific runtime code that implements the payout mechanism, and it is the calling logic's responsibility to process all returned reward entries.

`get_rewards_and_clear` can be called with any frequency, depending on how often reward payouts are to be performed. For Partner Chains using `pallet-sidechain`, it's convenient to put the payouts logic in the `OnNewEpoch` handler defined in the Sidechain pallet's `Config`.

## Configuration

The pallet uses the following configuration traits:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// Type representing a block beneficiary ID, which is credited with rewards for block production.
    type BeneficiaryId: Member + Parameter + MaxEncodedLen + Copy;
    
    /// Type representing block reward points.
    type BlockRewardPoints: Member + Parameter + AtLeast32BitUnsigned + MaxEncodedLen + Copy
        + BaseArithmetic + Default;

    /// Strategy for calculating block reward.
    type GetBlockRewardPoints: GetBlockRewardPoints<Self::BlockRewardPoints>;
}
```

## Storage

The pallet maintains the following storage items:

1. `Rewards`: A map of beneficiary IDs to their accumulated reward points
2. `PendingBlock`: The beneficiary ID for the current block in production (cleared after finalization)

## API Specification

### Extrinsics

- **set_current_block_beneficiary**: Sets the beneficiary for the current block

### Public Functions (API)

- **get_rewards_and_clear**: Returns all pending rewards and clears the storage

### Inherent Data

#### Inherent Identifier
```rust
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"beneficr";
```

#### Data Type
`T::BeneficiaryId` - The ID of the beneficiary who will receive the reward for the current block

#### Inherent Required
Yes, for every block. This ensures that every block has a designated beneficiary for rewards.

### Events

- `RewardsCollected(T::BeneficiaryId, T::BlockRewardPoints)`: Emitted when rewards are collected by a beneficiary

### Hooks

- `on_finalize`: Credits the pending block reward to the designated beneficiary when the block is finalized
- `on_initialize`: Prepares the pallet for a new block

## Configuration Example

```rust
// For a simple reward system with 1 point per block
parameter_types! {
    pub const BlockRewardAmount: Balance = 1;
}

impl pallet_block_rewards::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BeneficiaryId = AccountId;
    type BlockRewardPoints = Balance;
    type GetBlockRewardPoints = SimpleBlockCount;
}

// For a variable reward system
pub struct VariableBlockReward;
impl GetBlockRewardPoints<Balance> for VariableBlockReward {
    fn get_block_reward() -> Balance {
        // Custom logic to determine current block's reward value
        if block_number % 100 == 0 {
            10 // Higher reward every 100 blocks
        } else {
            1 // Regular reward
        }
    }
}

impl pallet_block_rewards::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BeneficiaryId = AccountId;
    type BlockRewardPoints = Balance;
    type GetBlockRewardPoints = VariableBlockReward;
}
```

## Integration Example

A typical integration will include:

1. Setting up the inherent data provider in the node service:

```rust
let inherent_data_providers = sp_inherents::InherentDataProviders::new();
inherent_data_providers
    .register_provider(BlockBeneficiaryInherentProvider::from_env("BLOCK_BENEFICIARY")?)
    .map_err(|e| format!("Failed to register inherent data provider: {:?}", e))?;
```

2. Implementing a reward processing mechanism in the runtime, typically called during epoch transitions:

```rust
impl pallet_sidechain::Config for Runtime {
    // ...other config items
    type OnNewEpoch = ProcessBlockRewards;
}

pub struct ProcessBlockRewards;
impl OnNewEpoch for ProcessBlockRewards {
    fn on_new_epoch(epoch_number: EpochNumber) {
        // Get and clear all accumulated rewards
        let rewards = BlockRewards::get_rewards_and_clear();
        
        // Process rewards (chain-specific logic)
        for (beneficiary, points) in rewards {
            // Convert points to actual token amounts
            let token_amount = calculate_token_amount(points);
            
            // Credit tokens to beneficiary
            Balances::deposit_creating(&beneficiary, token_amount);
        }
    }
}
```

## Dependencies

- frame_system
- frame_support
- sp_block_rewards (for inherent data handling)