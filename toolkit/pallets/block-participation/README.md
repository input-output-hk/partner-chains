# Block Participation Pallet

A Substrate pallet for tracking block production participation in partner chains.

## Overview

The Block Participation pallet provides functionality to track block production by validators in a partner chain. It allows for recording block authors and delegators associated with each block, enabling reward distribution and governance systems to account for validator participation.

## Purpose

This pallet serves as a vital component in partner chain ecosystems by:
1. Tracking which validators are actively producing blocks
2. Recording delegator participation in block production
3. Providing an interface for other pallets to query block production data
4. Managing the release of historical block production data when it's no longer needed

## Primitives

This pallet uses primitives defined in the Substrate blockchain framework along with custom imports:

```rust
use codec::{Decode, Encode};
use frame_support::traits::Get;
use frame_system::ensure_signed;
use scale_info::TypeInfo;
use sp_block_participation::*;
use sp_runtime::{
    traits::{Member, Parameter},
    RuntimeAppPublic,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*, vec};
```

## Configuration

This pallet has the following configuration trait:

```rust
pub trait Config: frame_system::Config {
    /// Weight information for extrinsics in this pallet
    type WeightInfo: crate::weights::WeightInfo;
    
    /// The type used to identify block authors
    type BlockAuthor: Member + Parameter + MaxEncodedLen;
    
    /// The type used to identify delegators
    type DelegatorId: Member + Parameter + MaxEncodedLen;
    
    /// A function that determines whether data for a specific slot should be released
    fn should_release_data(slot: Slot) -> Option<Slot>;
    
    /// A function that provides an iterator of blocks produced up to a given slot
    fn blocks_produced_up_to_slot(slot: Slot) -> impl Iterator<Item = (Slot, Self::BlockAuthor)>;
    
    /// A function that discards block production data up to a specified slot
    fn discard_blocks_produced_up_to_slot(slot: Slot);
    
    /// The inherent identifier used for this pallet
    const TARGET_INHERENT_ID: InherentIdentifier;
}
```

## API Specification

### Extrinsics

#### `note_processing`

Processes block production data from an inherent, recording block authors and delegator participation.

```rust
pub fn note_processing(origin: OriginFor<T>, data: InherentData) -> DispatchResult
```

### Public Functions

#### `should_release_data`

Determines if block production data should be released for a given slot.

```rust
pub fn should_release_data(slot: Slot) -> Option<Slot>
```

### Inherent Data

This pallet uses inherent data to provide block production information to the chain. The inherent data has the following structure:

```rust
pub struct InherentData {
    pub processed_up_to_slot: Slot,
    pub participation_maps: BTreeMap<Slot, ParticipationMap<BlockAuthor, DelegatorId>>,
}
```

## Integration

To integrate this pallet in your runtime:

1. Add the pallet to your runtime's `Cargo.toml`:
```toml
[dependencies]
pallet-block-participation = { version = "4.0.0-dev", default-features = false }
```

2. Implement the pallet's Config trait for your runtime:
```rust
impl pallet_block_participation::Config for Runtime {
    type WeightInfo = pallet_block_participation::weights::SubstrateWeight<Runtime>;
    type BlockAuthor = AccountId;
    type DelegatorId = AccountId;
    
    fn should_release_data(slot: Slot) -> Option<Slot> {
        // Your implementation
    }
    
    fn blocks_produced_up_to_slot(slot: Slot) -> impl Iterator<Item = (Slot, Self::BlockAuthor)> {
        // Your implementation
    }
    
    fn discard_blocks_produced_up_to_slot(slot: Slot) {
        // Your implementation
    }
    
    const TARGET_INHERENT_ID: InherentIdentifier = *b"blkparti";
}
```

3. Add the pallet to your runtime:
```rust
construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        // Other pallets
        BlockParticipation: pallet_block_participation::{Pallet, Call, Storage, Inherent},
    }
);
```

## Implementation Details

This pallet relies on external storage for tracking block production data. The actual storage of block authors and delegator participation is expected to be handled by the runtime implementation through the Config trait methods.

The pallet processes block production data through inherents, which should be created and supplied by the runtime during block production.
```

Here is the updated README.md for the Address Associations pallet (only minor changes needed):

```markdown
# Address Associations Pallet

A Substrate pallet for establishing and verifying associations between mainchain stake public keys and partner chain addresses.

## Overview

The Address Associations pallet enables validators to associate their mainchain stake public keys with their partner chain addresses. This association is critical for various cross-chain functionalities, particularly for tracking validator participation and distributing rewards.

## Purpose

This pallet serves as a bridge component in partner chain ecosystems by:
1. Allowing validators to create cryptographically verifiable links between their mainchain and partner chain identities
2. Providing a lookup mechanism to translate between mainchain and partner chain identities
3. Supporting various cross-chain functionalities including rewards distribution and governance

## Configuration

This pallet has the following configuration trait:

```rust
pub trait Config: frame_system::Config {
    /// Weight information for extrinsics in this pallet
    type WeightInfo: crate::weights::WeightInfo;
    
    /// The type used to represent a partner chain address
    type PartnerChainAddress: Member + Parameter + MaxEncodedLen;
    
    /// A function that returns the genesis UTXO ID
    fn genesis_utxo() -> UtxoId;
}
```

## Storage

This pallet defines the following storage items:

```rust
/// Maps a mainchain key hash to a partner chain address
pub type AddressAssociations<T: Config> = StorageMap<
    Hasher = Blake2_128Concat,
    Key = MainchainKeyHash,
    Value = T::PartnerChainAddress,
    QueryKind = OptionQuery,
>;
```

## API Specification

### Extrinsics

#### `associate_address`

Associates a mainchain stake public key with a partner chain address using a verifiable signature.

```rust
pub fn associate_address(
    origin: OriginFor<T>,
    mainchain_key: Vec<u8>,
    mainchain_signature: Vec<u8>
) -> DispatchResult
```

### Public Functions

#### `get_version`

Returns the version of the pallet.

```rust
pub fn get_version() -> Version
```

#### `get_all_address_associations`

Returns all mainchain key to partner chain address associations.

```rust
pub fn get_all_address_associations() -> Vec<(MainchainKeyHash, T::PartnerChainAddress)>
```

#### `get_partner_chain_address_for`

Returns the partner chain address associated with a given mainchain key hash.

```rust
pub fn get_partner_chain_address_for(mainchain_key_hash: &MainchainKeyHash) -> Option<T::PartnerChainAddress>
```

### Errors

- `MainchainKeyAlreadyAssociated`: Returned when attempting to associate a mainchain key that is already associated with an address
- `InvalidMainchainSignature`: Returned when the signature verification fails

### Events

Note: This pallet currently does not emit any events. This could be considered as a potential enhancement in future versions to improve traceability and integration capabilities.

## Usage

To use this pallet to associate a mainchain key with a partner chain address:

1. Create a signature using your mainchain stake private key on a specific message
2. Call the `associate_address` extrinsic with your mainchain public key and the signature
3. If the signature is valid and the key is not already associated, the association will be stored

The message to sign is constructed from:
- The genesis UTXO ID (provided by the runtime)
- The partner chain address of the caller

## Types

This pallet defines and uses the following types:

```rust
pub type MainchainKeyHash = [u8; 32];
pub type UtxoId = [u8; 32];
pub type Version = u32;
```

## Dependencies

This pallet depends on the following Substrate components:

- `frame_system`: For basic blockchain functionality
- `frame_support`: For various pallet utilities
- `sp_std`: For standard library types
- `sp_core`: For cryptographic utilities
- `sp_runtime`: For runtime types and traits
- `scale_info`: For type information

And also uses the following external crates:
- `codec`: For encoding and decoding
- `bitcoin`: For verification of Bitcoin-style signatures