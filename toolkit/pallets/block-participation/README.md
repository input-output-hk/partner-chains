# Block Participation Pallet

## Overview

The Block Participation pallet manages and tracks validator participation in the block production process across the network. It establishes a systematic mechanism for recording when and how validators contribute to consensus, which is essential for maintaining network security, fairness, and proper incentive distribution.

In the context of partner chains, "block participation" refers to the record of which validators have successfully produced blocks at which slots in the blockchain's history. This information is crucial for several reasons:

1. **Consensus Integrity**: By tracking participation, the system can verify that the consensus rules are being properly followed, and that block production responsibilities are correctly assigned and fulfilled.

2. **Performance Monitoring**: The participation data enables the network to monitor the performance of validators over time, identifying those who consistently meet their obligations and those who don't.

3. **Rewards**: Accurate participation records provide the foundation for fairly distributing rewards to validators who contribute to network security.

4. **Historical Analysis**: Participation data offers valuable insights into network health and validator behavior patterns over time, which can inform governance decisions and protocol improvements.

5. **Slot Finality**: By knowing when participation data has been processed up to a certain slot, the system can make determinations about when certain slots can be considered "finalized" from a participation tracking perspective.

The pallet provides a time-windowed approach to participation tracking, where data is processed up to specific slots, and historical data beyond a configured slack window can be safely released to optimize storage usage. This creates an efficient balance between maintaining necessary historical records and managing chain state growth.

Additionally, it integrates with the inherent data mechanism of Substrate, allowing participation processing to be included automatically in blocks when appropriate, rather than requiring explicit extrinsic calls for every update.

By separating the concerns of recording participation (typically handled by the block production log pallet) and processing that participation data (handled by this pallet), the system achieves a clean architectural separation that enhances maintainability and allows for more flexible reward and governance mechanisms.

## Purpose

This pallet serves several important purposes in the partner chain ecosystem:

- Tracks the processing progress of block participation data
- Determines when historical block production data should be released
- Helps maintain a record of validator participation in the consensus process
- Supports reward mechanisms based on participation history
- Enables other pallets to query if participation data is ready to be released or processed

## Primitives

The Block Participation pallet utilizes several primitive types and structures defined in the `toolkit/primitives/block-participation` crate.

## Configuration

The pallet uses the following configuration traits:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// The slot type
    type Slot: Member + Parameter + AtLeast32BitUnsigned + Default + Copy + TypeInfo;

    /// The slack window, in number of slots, that determines how long to wait before releasing block
    /// production data.
    #[pallet::constant]
    type SlackWindow: Get<u32>;
}
```

## Storage

The pallet maintains one main storage item:

- `ProcessedUpToSlot`: Records the slot up to which participation data has been processed

## API Specification

### Extrinsics

- **note_processing**: Records that block participation data has been processed up to a specific slot

### Public Functions (API)

- **should_release_data**: Returns the slot up to which block production data should be released, or None

### Inherent Data

#### Inherent Identifier
```rust
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"partcptn";
```

#### Data Type
`Slot` - A specific slot boundary up to which block participation data should be processed

#### Inherent Required
Yes, when participation data needs to be processed. The runtime verifies this inherent data by checking:
- If a previous inherent was already processed in the same block
- Whether the slot value is greater than the last processed slot

### Events

- `Processed(T::Slot)`: Emitted when block participation data has been processed up to a specific slot

### Errors

- `ProcessingInvalidPreviousSlot`: The processing operation would move the processed slot boundary backwards, which is not allowed
- `AlreadyProcessedInBlock`: Block participation data has already been processed in the current block

## Hooks

The Block Participation pallet implements the following FRAME hooks:

### on_initialize

The `on_initialize` hook is called at the beginning of each block's execution, before any extrinsics are processed. For the Block Participation pallet, this hook serves several important purposes:

```rust
fn on_initialize(n: BlockNumberFor<T>) -> Weight {
   // Function implementation
}
```

## Usage

This pallet works in conjunction with other pallets that track block production and validator participation. To use it:

1. At regular intervals (typically determined by epoch boundaries), submit the `note_processing` extrinsic to record that participation data has been processed up to a specific slot.

2. Other pallets can call `should_release_data` to determine if historical participation data can be released for a given slot, based on the slack window configuration.

## Integration with Block Production Log

This pallet is typically used in conjunction with the Block Production Log pallet, which records the actual block production data. Together, they provide a complete system for:

1. Recording which validators produced blocks in which slots
2. Tracking when this data has been processed (e.g., for rewards calculation)
3. Determining when historical data can be safely released to reclaim storage

## Configuration Examples

A typical configuration might use a slack window of 100 slots:

```rust
parameter_types! {
    pub const ParticipationSlackWindow: u32 = 100;
}

impl pallet_block_participation::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Slot = u64;
    type SlackWindow = ParticipationSlackWindow;
}
```

## Dependencies

- frame_system
- frame_support
- sp_block_participation (for inherent data handling)