
# Session Runtime Stub Pallet

## Overview

The Session Runtime Stub Pallet provides compatibility utilities that bridge the gap between Substrate's original session pallet and the partner chain's custom session management. This pallet serves as an adapter or stub implementation, allowing runtimes that use the Partner Chains Session pallet to seamlessly integrate with components that expect the standard Substrate Session pallet interface.

In the Substrate ecosystem, the session pallet is a fundamental component that many other pallets and consensus mechanisms depend on. However, partner chains need custom session management functionality tailored to their specific requirements. The Session Runtime Stub pallet solves this compatibility challenge by providing default implementations for the standard Session pallet traits while delegating the actual session management logic to the Partner Chains Session pallet.

This approach offers several advantages:

1. **Compatibility Preservation**: Ensures that partner chain runtimes remain compatible with existing Substrate components that depend on the standard Session pallet.

2. **Implementation Simplification**: Reduces boilerplate code by providing ready-to-use implementations of the standard Session pallet traits.

3. **Clean Architecture**: Maintains a clear separation between the partner chain's custom session management and the compatibility layer needed for the broader Substrate ecosystem.

4. **Minimal Overhead**: Adds just enough code to satisfy interface requirements without introducing redundant functionality.

The pallet is intentionally lightweight, focusing only on providing stub implementations of traits and a convenient macro for implementing the standard Session pallet's configuration trait.

## Purpose

This pallet serves as a bridge between the Partner Chains Session pallet and components that expect the standard Substrate Session pallet interface. Its key purposes include:

- Providing stub implementations of standard Session pallet traits
- Simplifying the configuration of runtimes that need to implement both session pallet interfaces
- Ensuring compatibility with existing Substrate components
- Reducing boilerplate code in partner chain runtimes

## Configuration

This pallet does not have a traditional configuration trait. Instead, it provides implementations of several trait interfaces and a macro to simplify runtime configuration.

## API Specification

### Extrinsics

This pallet does not expose any extrinsics. It serves primarily as a utility for configuration and trait implementations.

### Public Functions (API)

The pallet provides the following implementations:

- **PalletSessionStubImpls**: A struct providing stub implementations for Session pallet traits
  - Implements `ShouldEndSession` - Always returns false to delegate session ending to Partner Chains Session
  - Implements `SessionManager` - Provides empty implementations that delegate to Partner Chains Session
  - Implements `Convert<T, Option<T>>` - Always returns Some(t) for validator ID conversion

### Macros

- **impl_pallet_session_config**: A macro that implements the standard `pallet_session::Config` trait for a runtime that already implements `pallet_partner_chains_session::Config`

```rust
// Example usage:
impl_pallet_session_config!(Runtime);
```

This macro automatically creates an implementation of `pallet_session::Config` that reuses types and implementations from the runtime's existing `pallet_partner_chains_session::Config` implementation.

## Integration Example

A typical integration in a runtime would look like:

```rust
// First implement the Partner Chains Session configuration
impl pallet_partner_chains_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ShouldEndSession = ValidatorManagementSessionManager<Self>;
    type NextSessionRotation = ValidatorManagementSessionManager<Self>;
    type SessionManager = ValidatorManagementSessionManager<Self>;
    type SessionHandler = (Aura, Grandpa);
    type Keys = SessionKeys;
}

// Then use the macro to implement the standard Session configuration
pallet_session_runtime_stub::impl_pallet_session_config!(Runtime);
```

This approach allows the runtime to focus on implementing the partner chain's session management logic while automatically gaining compatibility with components that expect the standard Session pallet interface.

## Usage with Other Pallets

Many Substrate pallets and consensus mechanisms expect a runtime to implement `pallet_session::Config`. The Session Runtime Stub pallet allows partner chain runtimes to satisfy these dependencies without duplicating code or introducing inconsistencies:

1. **Consensus Mechanisms**: Modules like BABE, GRANDPA, and others often require Session pallet integration
2. **Staking Pallet**: Depends on the Session pallet for validator set management
3. **Other Core Substrate Pallets**: May have indirect dependencies on the Session pallet

By using this stub pallet, partner chain runtimes can maintain compatibility with these components while using their custom Partner Chains Session pallet for actual session management.

## Dependencies

- sp_staking
- sp_std
- pallet_session (for trait definitions)
- pallet_partner_chains_session (for integration)
