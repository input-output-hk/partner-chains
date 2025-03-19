# Address Associations Pallet

## Overview

The Address Associations pallet provides functionality to associate mainchain (i.e., Cardano) stake public keys with partner chain addresses. This forms a critical link between the main chain and the partner chain, enabling cross-chain identity verification and operations.

## Purpose

This pallet serves several purposes:
- Establishes verifiable links between mainchain identities and partner chain addresses
- Enables cross-chain validation of key ownership through cryptographic signatures
- Provides a foundation for permission-based operations that require mainchain identity verification
- Supports cross-chain governance and validator selection processes

## Configuration

The pallet uses the following configuration traits:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// Type representing the partner chain address.
    type PartnerChainAddress: Parameter + Member + MaxEncodedLen + Copy + TypeInfo;

    /// Type representing the mainchain stake public key.
    type StakePublicKey: Parameter + Member + MaxEncodedLen + TypeInfo;
}
```

## Storage

The pallet maintains two main storage maps:

1. `StakePublicKeyToPartnerChainAddress`: Maps from a mainchain stake public key to a partner chain address
2. `PartnerChainAddressToStakePublicKey`: Maps from a partner chain address to a mainchain stake public key

## API Specification

### Extrinsics

- **associate_address**: Associates a mainchain public key with a partner chain address

### Public Functions (API)

- **get_version**: Returns the current pallet version
- **get_all_address_associations**: Returns an iterator over all mainchain-partnerchain address associations
- **get_partner_chain_address_for**: Returns the partner chain address for a given mainchain public key if it exists

### Inherent Data

This pallet does not use inherent data.

### Events

- `AddressAssociated(T::StakePublicKey, T::PartnerChainAddress)`: Emitted when a mainchain public key is successfully associated with a partner chain address

### Errors

- `AddressAlreadyAssociated`: The partner chain address is already associated with a different stake public key
- `StakePublicKeyAlreadyAssociated`: The stake public key is already associated with a different partner chain address
- `InvalidSignature`: The provided signature is invalid and cannot prove ownership of the stake key

## Usage

To associate a mainchain stake public key with a partner chain address, the caller must:

1. Generate a signature using their mainchain stake key
2. Submit the `associate_address` extrinsic with their partner chain address, the signature, and their stake public key

The pallet verifies the signature to ensure the caller owns both the mainchain stake key and the partner chain address before creating the association.

## Dependencies

- frame_system
- frame_support

## Security Considerations

- Signatures are verified cryptographically to prevent unauthorized associations
- Once created, associations cannot be modified without governance intervention
- The pallet ensures one-to-one mappings between stake public keys and partner chain addresses