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

The Sidechain pallet relies on primitives defined in the Substrate blockchain framework along with custom imports:

```rust
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::BlockNumberFor;
use sidechain_domain::UtxoId;
use sidechain_domain::{ScEpochNumber, ScSlotNumber};
use sp_sidechain::OnNewEpoch;
```

## Configuration

This pallet has the following configuration trait:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    fn current_slot_number() -> ScSlotNumber;
    type OnNewEpoch: OnNewEpoch;
}
```

Where:
- `current_slot_number()`: Function that returns the current slot number
- `OnNewEpoch`: Trait that defines behavior to be executed when a new epoch begins

## Storage

The pallet maintains several storage items:

```rust
#[pallet::storage]
pub(super) type EpochNumber<T: Config> = StorageValue<_, ScEpochNumber, ValueQuery>;

#[pallet::storage]
pub(super) type SlotsPerEpoch<T: Config> =
    StorageValue<_, sidechain_slots::SlotsPerEpoch, ValueQuery>;

#[pallet::storage]
pub(super) type GenesisUtxo<T: Config> = StorageValue<_, UtxoId, ValueQuery>;
```

These storage items track:
1. `EpochNumber`: The current epoch number
2. `SlotsPerEpoch`: The number of slots in each epoch
3. `GenesisUtxo`: The genesis UTXO that established the sidechain

## API Specification

### Extrinsics

The Sidechain pallet does not expose direct extrinsics. Epoch transitions and related operations happen automatically through the hooks mechanism.

### Public Functions

```rust
pub fn genesis_utxo() -> UtxoId {
    GenesisUtxo::<T>::get()
}

pub fn current_epoch_number() -> ScEpochNumber {
    let current_slot = T::current_slot_number();
    let slots_per_epoch = Self::slots_per_epoch();
    slots_per_epoch.epoch_number_from_sc_slot(current_slot)
}

pub fn slots_per_epoch() -> sidechain_slots::SlotsPerEpoch {
    SlotsPerEpoch::<T>::get()
}
```

These functions provide access to:
- The genesis UTXO that established the sidechain
- The current epoch number (calculated from current slot and slots per epoch)
- The configured number of slots per epoch

### Events

The Sidechain pallet does not emit events directly, but it triggers the OnNewEpoch handlers when epochs change.

### Errors

The Sidechain pallet does not define any custom errors.

## Hooks

The pallet primarily operates through its hooks:

```rust
fn on_initialize(n: BlockNumberFor<T>) -> Weight {
    let real_epoch = Self::current_epoch_number();

    match EpochNumber::<T>::try_get().ok() {
        Some(saved_epoch) if saved_epoch != real_epoch => {
            log::info!("⏳ New epoch {real_epoch} starting at block {:?}", n);
            EpochNumber::<T>::put(real_epoch);
            <T::OnNewEpoch as OnNewEpoch>::on_new_epoch(saved_epoch, real_epoch)
                .saturating_add(T::DbWeight::get().reads_writes(2, 1))
        },
        None => {
            log::info!("⏳ Initial epoch {real_epoch} starting at block {:?}", n);
            EpochNumber::<T>::put(real_epoch);
            T::DbWeight::get().reads_writes(2, 1)
        },
        _ => T::DbWeight::get().reads_writes(2, 0),
    }
}
```

This hook runs at the beginning of each block and:
1. Calculates the current epoch based on the current slot
2. Compares it with the stored epoch number
3. If different (or not initialized), updates the stored epoch and calls OnNewEpoch handlers
4. If unchanged, does nothing substantial
5. Returns appropriate weight for the operations performed

## Genesis Configuration

The pallet requires the following genesis configuration:

```rust
#[pallet::genesis_config]
#[derive(frame_support::DefaultNoBound)]
pub struct GenesisConfig<T: Config> {
    pub genesis_utxo: UtxoId,
    pub slots_per_epoch: sidechain_slots::SlotsPerEpoch,
    #[serde(skip)]
    pub _config: sp_std::marker::PhantomData<T>,
}

#[pallet::genesis_build]
impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
    fn build(&self) {
        GenesisUtxo::<T>::put(self.genesis_utxo);
        SlotsPerEpoch::<T>::put(self.slots_per_epoch);
    }
}
```

This configuration initializes:
- The genesis UTXO that links the sidechain to its parent chain
- The number of slots per epoch that defines the temporal structure

## Integration

To integrate this pallet in your runtime:

1. Add the pallet to your runtime's `Cargo.toml`:
```toml
[dependencies]
pallet-sidechain = { version = "1.6.0", default-features = false }
```

2. Implement the pallet's Config trait for your runtime:
```rust
impl pallet_sidechain::Config for Runtime {
    fn current_slot_number() -> ScSlotNumber {
        // Provide implementation to fetch current slot, typically from a slot provider
        Slots::current_slot()
    }
    
    type OnNewEpoch = EpochTransitionHandlers;
}
```

3. Define your OnNewEpoch handler:
```rust
pub struct EpochTransitionHandlers;
impl sp_sidechain::OnNewEpoch for EpochTransitionHandlers {
    fn on_new_epoch(old_epoch: ScEpochNumber, new_epoch: ScEpochNumber) -> Weight {
        // Perform epoch transition actions
        let mut weight = Weight::zero();
        
        // Example: Distribute rewards from the previous epoch
        weight = weight.saturating_add(BlockRewards::process_epoch_rewards(old_epoch));
        
        // Example: Update validator set for the new epoch
        weight = weight.saturating_add(SessionValidatorManagement::update_validators(new_epoch));
        
        weight
    }
}
```

4. Add the pallet to your runtime:
```rust
construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        // Other pallets
        Sidechain: pallet_sidechain::{Pallet, Storage, Config<T>},
    }
);
```

5. Configure genesis parameters in your chain spec:
```rust
pallet_sidechain: SidechainConfig {
    genesis_utxo: [0u8; 32].into(),  // Replace with actual genesis UTXO
    slots_per_epoch: 432000.into(),  // Example: 5 days at 1 second per slot
    _config: Default::default(),
},
```

Relationships between the `sidechain` pallet and other pallets in the system:

```mermaid

```

## Usage

The Sidechain pallet is typically used as a core coordination mechanism. The typical usage flow is:

1. At runtime initialization, the pallet is configured with genesis parameters that establish the epoch structure.

2. For each block, the `on_initialize` hook automatically checks if an epoch transition has occurred.

3. When an epoch transition is detected, the configured OnNewEpoch handler is called, which typically:
    - Updates validator sets through session management
    - Processes accumulated rewards
    - Updates protocol parameters
    - Performs other epoch-based administrative tasks

4. Other pallets can query the current epoch or slots per epoch to coordinate their own activities.

## Integration with Other Pallets

The Sidechain pallet serves as a core coordination mechanism for other pallets through the OnNewEpoch trait. Typical integrations include:

1. **Session Validator Management**: Coordinating validator set rotations with epoch boundaries
2. **Rewards Distribution**: Processing accumulated rewards at epoch transitions
3. **Parameter Updates**: Applying new protocol parameters at epoch boundaries

This separation of concerns creates a clean architecture that decouples temporal management from the specific behaviors that need to be coordinated at epoch transitions.