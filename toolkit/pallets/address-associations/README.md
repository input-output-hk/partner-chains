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
    Key = MainchainKeyHash,
    Value = T::PartnerChainAddress,
    QueryKind = OptionQuery,
>;
```

This maps from a mainchain key hash to a partner chain address.

## API Specification

### Extrinsics

- **associate_address**: Associates a mainchain public key with a partner chain address
  ```rust
  pub fn associate_address(
      _origin: OriginFor<T>,
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

### Inherent Data

This pallet does not use inherent data.

### Events

This pallet currently does not emit any events.

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

The pallet verifies the signature to ensure the caller owns the mainchain stake key before creating the association.

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