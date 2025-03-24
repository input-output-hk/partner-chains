# Block Production Log Pallet

## Overview

The Block Production Log pallet records and maintains the historical data of which validators have produced blocks at specific slots throughout the blockchain's lifetime. This chronological record serves as the source of truth for validator participation in the network's consensus process.

Unlike many other pallets that focus on state management, the Block Production Log specializes in historical record-keeping - capturing the essential relationship between time (slots) and validator activity. This historical data forms the empirical basis for several critical network functions:

1. **Validator Performance Assessment**: By maintaining an accurate log of which validators successfully produced blocks when scheduled, the system can evaluate the reliability and performance of validators over time. This assessment is crucial for governance decisions regarding validator selection and rewards.

2. **Reward Distribution Fairness**: Rewards in proof-of-stake networks are typically distributed based on validator participation. The block production log provides the verifiable data needed to ensure rewards are distributed fairly according to actual contributions.

3. **Network Liveness Analysis**: The record of block production over time enables analysis of network liveness and the effectiveness of the validator set in maintaining consistent block production.

4. **Consensus Mechanism Verification**: The block production log provides evidence that the consensus mechanism is functioning as expected, with validators producing blocks according to the protocol rules.

5. **Delegator Information**: For delegated proof-of-stake systems, the log informs delegators about validator performance, helping them make informed decisions about delegation.

The pallet implements an efficient storage strategy by supporting three key operations:
- Appending new block production records
- Retrieving historical records up to a specified slot
- Removing (pruning) historical records that are no longer needed

This approach balances the need for historical record-keeping with efficient storage management. By allowing controlled pruning of old data once it has been processed (typically for reward calculations), the pallet prevents unbounded state growth while ensuring all necessary historical data is available when needed.

The Block Production Log pallet works seamlessly with the Block Participation pallet, which determines when historical data has been fully processed and can safely be pruned from storage.

## Purpose

This pallet serves several important purposes in the partner chain ecosystem:

- Maintains a chronological record of block production by validators
- Provides historical data for reward calculations
- Supports analysis of validator performance over time
- Enables efficient pruning of historical data to manage state growth
- Forms the foundation for fair reward distribution based on participation

## Primitives

The Block Production Log pallet relies on primitives defined in the `toolkit/primitives/block-production-log` crate.

## Configuration

The pallet uses the following configuration traits:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// Type representing a block producer ID, which is recorded in the block production log.
    type BlockProducerId: Parameter + Member + Copy + MaybeSerializeDeserialize + Debug + MaxEncodedLen
    + TypeInfo + Ord;

    /// Type representing a block slot.
    type Slot: Parameter + Member + Copy + AtLeast32BitUnsigned + MaybeSerializeDeserialize
    + Default + Debug + TypeInfo + Ord;
}
```

## Storage

The pallet maintains several storage items:

1. `BlockProductionLogEntries`: A map of slots to block producers who created blocks at those slots
2. `BlockProductionLogBoundary`: Optional slot boundary marking the earliest slot in the log

## API Specification

### Extrinsics

- **append**: Appends the block producer to the production log
  ```rust
  pub fn append(origin: OriginFor<T>, block_producer: T::BlockProducerId, slot: T::Slot) -> DispatchResult
  ```

### Public Functions (API)

- **take_prefix**: Returns and removes block production data up to the given slot
  ```rust
  pub fn take_prefix(slot: T::Slot) -> Vec<(T::Slot, T::BlockProducerId)>
  ```

- **peek_prefix**: Returns an iterator of block production data up to the given slot without removing it
  ```rust
  pub fn peek_prefix(slot: T::Slot) -> Vec<(T::Slot, T::BlockProducerId)>
  ```

- **drop_prefix**: Removes block production data up to the given slot
  ```rust
  pub fn drop_prefix(slot: T::Slot) -> DispatchResult
  ```

### Inherent Data

#### Inherent Identifier
```rust
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"blprdlog";
```

#### Data Type
`T::BlockProducerId` - The ID of the block producer who created the current block

#### Inherent Required
Yes, when a block is produced. The pallet verifies this inherent data to ensure blocks include information about who produced them.

#### Inherent Verification
The pallet verifies that the provided block producer ID matches the expected producer for the current slot. If verification fails, block production is halted with an appropriate error, ensuring that only valid block production records are added to the log.

### Events

- `Appended(T::BlockProducerId, T::Slot)`: Emitted when a block producer is appended to the log for a specific slot. This event is triggered during regular block production.
- `Dropped(T::Slot)`: Emitted when production data is dropped up to a specific slot. This typically occurs after data has been processed for rewards and is no longer needed.

### Errors

- `NoBlocksToTake`: Attempted to take blocks but no blocks were available in the specified range
- `InvalidSlotBoundary`: Attempted to set an invalid slot boundary in the block production log

## Hooks

The Block Production Log pallet implements the following FRAME hooks to ensure proper handling of block production records:

### on_initialize

The `on_initialize` hook is called at the beginning of each block's execution, before any extrinsics are processed. For the Block Production Log pallet, this hook performs essential setup for inherent data verification:

```rust
fn on_initialize(n: BlockNumberFor<T>) -> Weight {
    // Sets up the verification context for the current block
    // Initializes the expected block producer for the current slot
    // Returns the appropriate weight for this operation
}
```

This hook ensures that each block's inherent data can be properly verified against the expected block production schedule.

## Usage

The Block Production Log pallet is typically used in conjunction with the consensus mechanism and block participation tracking. The typical usage flow is:

1. For each block, the `append` function is called (usually via inherent data) to record which validator produced the block at the current slot.

2. Periodically, other pallets (such as a rewards pallet) can call `peek_prefix` to examine historical block production data without removing it.

3. After historical data has been fully processed (typically determined by the Block Participation pallet), the `drop_prefix` function can be called to prune old data and manage storage growth.

4. The `take_prefix` function combines retrieval and pruning in a single operation for cases where data will be processed immediately and then no longer needed.

## Integration with Block Participation

This pallet is designed to work closely with the Block Participation pallet:

1. The Block Production Log pallet maintains the raw history of block production.
2. The Block Participation pallet tracks when this history has been processed (e.g., for rewards).
3. Once processed, the Block Participation pallet signals that historical data can be pruned via the `drop_prefix` function.

This separation of concerns creates a clean architecture that separates record-keeping from record processing.

## Configuration Example

```rust
impl pallet_block_production_log::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BlockProducerId = AccountId;
    type Slot = u64;
}
```

## Dependencies

- frame_system
- frame_support
- sp_block_production_log (for inherent data handling)
```

```markdown
# Address Associations Pallet

## Overview

The Address Associations pallet provides functionality to associate mainchain (i.e., Cardano) stake public keys with partner chain addresses. This forms a critical link between the main chain and the partner chain, enabling cross-chain identity verification and operations.

## Purpose

This pallet serves several purposes:
- Establishes verifiable links between mainchain identities and partner chain addresses
- Enables cross-chain validation of key ownership through cryptographic signatures
- Provides a foundation for permission-based operations that require mainchain identity verification

## Configuration

The pallet uses the following configuration traits:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// Weight information for extrinsics in this pallet
    type WeightInfo: crate::weights::WeightInfo;

    /// Partner chain address type
    type PartnerChainAddress: Member + Parameter + MaxEncodedLen;

    /// Function to provide the genesis UTXO ID
    fn genesis_utxo() -> UtxoId;
}
```

## Storage

The pallet maintains a single storage map:

```rust
#[pallet::storage]
pub type AddressAssociations<T: Config> = StorageMap<
    Hasher = Blake2_128Concat,
    Key = MainchainKeyHash,  // Derived from the StakePublicKey
    Value = T::PartnerChainAddress,
    QueryKind = OptionQuery,
>;
```

This maps from a mainchain key hash (derived from a stake public key) to a partner chain address.

## API Specification

### Extrinsics

- **associate_address**: Associates a mainchain public key with a partner chain address
  ```rust
  pub fn associate_address(
      origin: OriginFor<T>,
      partnerchain_address: T::PartnerChainAddress,
      signature: StakeKeySignature,
      stake_public_key: StakePublicKey,
  ) -> DispatchResult
  ```

### Public Functions (API)

- **get_version**: Returns the current pallet version
  ```rust
  pub fn get_version() -> u32
  ```

- **get_all_address_associations**: Returns an iterator over all mainchain-partnerchain address associations
  ```rust
  pub fn get_all_address_associations() -> impl Iterator<Item = (MainchainKeyHash, T::PartnerChainAddress)>
  ```

- **get_partner_chain_address_for**: Returns the partner chain address for a given mainchain public key if it exists
  ```rust
  pub fn get_partner_chain_address_for(stake_public_key: &StakePublicKey) -> Option<T::PartnerChainAddress>
  ```

- **get_mainchain_key_for**: Returns the mainchain key hash for a given partner chain address if it exists
  ```rust
  pub fn get_mainchain_key_for(address: &T::PartnerChainAddress) -> Option<MainchainKeyHash>
  ```

### Inherent Data

This pallet does not use inherent data.

### Events

The pallet emits the following events:

```rust
#[pallet::event]
#[pallet::generate_deposit(pub(super) fn deposit_event)]
pub enum Event<T: Config> {
    /// A new association between a mainchain key and partner chain address was created
    AddressAssociated {
        mainchain_key_hash: MainchainKeyHash,
        partner_chain_address: T::PartnerChainAddress,
    },
}
```

### Errors

```rust
#[pallet::error]
pub enum Error<T> {
    MainchainKeyAlreadyAssociated,
    InvalidMainchainSignature,
}
```

- `MainchainKeyAlreadyAssociated`: The mainchain key is already associated with a partner chain address
- `InvalidMainchainSignature`: The provided signature is invalid and cannot prove ownership of the stake key

## Usage

To associate a mainchain stake public key with a partner chain address, the caller must:

1. Generate a signature using their mainchain stake key over a message containing:
    - The stake public key
    - The partner chain address
    - The genesis UTXO ID
2. Submit the `associate_address` extrinsic with their partner chain address, the signature, and their stake public key

The pallet verifies the signature to ensure the caller owns the mainchain stake key before creating the association. Upon successful verification, the pallet:
1. Creates the association in storage
2. Emits an `AddressAssociated` event

## Types

The pallet relies on several types imported from the `sidechain_domain` crate:
- `StakePublicKey`: Represents a mainchain stake public key
- `MainchainKeyHash`: A hash of a mainchain stake public key
- `StakeKeySignature`: A signature created using a mainchain stake key
- `UtxoId`: Identifier for a UTXO (Unspent Transaction Output)

## Dependencies

- frame_system
- frame_support
- sidechain_domain (for mainchain types)

## Security Considerations

- Signatures are verified cryptographically to prevent unauthorized associations
- Once created, associations cannot be modified without governance intervention
- The pallet ensures that a mainchain key can only be associated with one partner chain address