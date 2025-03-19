# Sidechain Pallet

## Overview

The Sidechain pallet serves as the foundational temporal management component for partner chains, providing core functionality for tracking and coordinating epoch transitions. This pallet acts as the authoritative time-keeper for the partner chain ecosystem, establishing the connection between slots, blocks, and epochs that governs the rhythm of the entire blockchain.

At its essence, the Sidechain pallet handles the critical responsibilities of:

1. **Epoch Management**: The pallet maintains the current epoch number and provides mechanisms to detect epoch transitions. Epochs represent fundamental time periods in the blockchain, during which validator committees remain constant and certain protocol parameters are fixed.

2. **Slot Configuration**: By storing the number of slots per epoch, the pallet establishes the temporal structure of the chain. This configuration determines how frequently epochs change and affects the cadence of validator rotations and other epoch-based processes.

3. **Genesis Information**: The pallet stores essential genesis information, particularly the genesis UTXO that establishes the link between the partner chain and its parent chain (typically Cardano).

4. **Temporal Coordination**: Perhaps most importantly, the Sidechain pallet coordinates epoch-based activities across the entire blockchain. When an epoch transition occurs, the pallet triggers notifications to other components through the OnNewEpoch mechanism, enabling coordinated actions such as validator set rotations, reward distributions, and parameter updates.

Unlike regular transaction-processing pallets, the Sidechain pallet's functionality operates primarily through its hooks and callbacks. It monitors the passage of slots and automatically triggers epoch transitions when appropriate, without requiring external extrinsic calls to manage this process.

This design makes the Sidechain pallet the heartbeat of the partner chain, providing a reliable temporal framework that other pallets can depend upon for scheduling their operations. It creates a foundation for orderly epoch transitions that maintain consensus stability while allowing for dynamic updates to the validator set and other parameters.

## Purpose

The Sidechain pallet serves several critical purposes in the partner chain ecosystem:

- Maintains the authoritative record of the current epoch
- Detects and coordinates epoch transitions across the runtime
- Stores essential configuration like slots per epoch
- Provides access to genesis information, particularly the genesis UTXO
- Enables other pallets to reliably schedule activities based on epochs
- Notifies subscribed components when new epochs begin

## Primitives

The Sidechain pallet relies on primitives defined in the `toolkit/primitives/sidechain` crate.

## Configuration

The pallet uses the following configuration traits:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    fn current_slot_number() -> ScSlotNumber;
    type OnNewEpoch: OnNewEpoch;
}
```

## Storage

The pallet maintains several storage items:

1. `EpochNumber`: The current epoch number
2. `SlotsPerEpoch`: The number of slots in each epoch
3. `GenesisUtxo`: The genesis UTXO that established the sidechain

## API Specification

### Extrinsics

The Sidechain pallet does not expose direct extrinsics. Epoch transitions and related operations happen automatically through the hooks mechanism.

### Public Functions (API)

- **genesis_utxo**: Returns the genesis UTXO
- **current_epoch_number**: Returns the current epoch number
- **slots_per_epoch**: Returns the number of slots per epoch

### Inherent Data

The Sidechain pallet does not use inherent data directly.

### Events

The Sidechain pallet does not emit events directly, but it triggers the OnNewEpoch handlers when epochs change.

## Hooks

The pallet primarily operates through its hooks:

- **on_initialize**: Called at the beginning of each block. This hook is where epoch transitions are detected by comparing the real epoch (calculated from the current slot) with the stored epoch. When a transition is detected, the OnNewEpoch handlers are called.

## Genesis Configuration

The pallet requires the following genesis configuration:

```rust
pub struct GenesisConfig<T: Config> {
    pub genesis_utxo: UtxoId,
    pub slots_per_epoch: sidechain_slots::SlotsPerEpoch,
}
```

## Integration with Other Pallets

The Sidechain pallet serves as a core coordination mechanism for other pallets through the OnNewEpoch trait. Common integrations include:

1. **Session Validator Management**: Coordinating validator set rotations with epoch boundaries
2. **Rewards Distribution**: Processing accumulated rewards at epoch transitions
3. **Parameter Updates**: Applying new protocol parameters at epoch boundaries

A typical integration might look like:

```rust
pub struct EpochTransitionHandlers;
impl sp_sidechain::OnNewEpoch for EpochTransitionHandlers {
    fn on_new_epoch(old_epoch: ScEpochNumber, new_epoch: ScEpochNumber) -> Weight {
        // Perform epoch transition actions
        let mut weight = Weight::zero();
        
        // Distribute rewards from the previous epoch
        weight = weight.saturating_add(BlockRewards::process_epoch_rewards(old_epoch));
        
        // Update protocol parameters for the new epoch
        weight = weight.saturating_add(Parameters::update_for_epoch(new_epoch));
        
        weight
    }
}

impl pallet_sidechain::Config for Runtime {
    fn current_slot_number() -> ScSlotNumber {
        Slots::current_slot()
    }
    type OnNewEpoch = EpochTransitionHandlers;
}
```

## Dependencies

- frame_system
- frame_support
- sp_sidechain (for the OnNewEpoch trait)
- sidechain_domain (for domain-specific types like UtxoId)
- sidechain_slots (for slot-related types)