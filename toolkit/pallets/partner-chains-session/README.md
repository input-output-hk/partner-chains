# Partner Chains Session Pallet

## Overview

The Partner Chains Session pallet is a session management component designed specifically for the partner chains ecosystem. It is a refined adaptation of Substrate's standard session pallet, customized to meet the unique requirements of partner chains while maintaining compatibility with core Substrate functionalities.

This pallet serves as the cornerstone for managing validator sets and their associated session keys within the partner chain network. Unlike the original session pallet, it has been streamlined to eliminate unnecessary complexities while enhancing features that are critical for partner chains' consensus mechanisms.

The Partner Chains Session pallet operates on the principle of session-based validator rotation, where:

1. **Session Management**: A "session" represents a period during which a fixed set of validators is responsible for block production and finality. The pallet handles the orderly transition between sessions, ensuring consensus continuity.

2. **Key Management**: Validators require various cryptographic keys for different consensus-related functions. The pallet provides a framework for validators to register and update their session keys securely.

3. **Validator Rotation**: At the end of each session, the pallet can rotate to a new validator set, allowing the network to dynamically adjust its consensus participants based on various selection criteria.

4. **Validator Disabling**: The pallet includes mechanisms to disable validators during a session if they're found to be misbehaving, enhancing network security and resilience.

This pallet eliminates the complex queuing mechanism from the original session pallet, opting instead for a direct rotation model that aligns better with the epoch-based consensus used in partner chains. This design choice reduces state bloat and simplifies the mental model for developers working with the pallet.

Furthermore, the pallet is designed to integrate seamlessly with other partner chain components, particularly the Session Validator Management pallet, creating a cohesive system for validator selection, rotation, and session management.

## Purpose

The Partner Chains Session pallet fulfills several critical functions within the partner chain ecosystem:

- Manages the lifecycle of validator sessions, including session creation, rotation, and termination
- Provides a secure mechanism for validators to associate their identity with necessary session keys
- Facilitates the orderly transition between validator sets without disrupting consensus
- Enables on-the-fly disabling of validators who fail to perform their duties
- Serves as the interface between validator selection logic and the consensus mechanism
- Maintains backward compatibility with Substrate components that expect the standard session pallet interface

## Primitives

The Partner Chains Session pallet relies on primitives defined in the `toolkit/primitives/session-manager` crate.

## Configuration

The pallet uses the following configuration traits:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type RuntimeEvent: From<Event> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// A stable ID for a validator.
    type ValidatorId: Member
        + Parameter
        + MaybeSerializeDeserialize
        + MaxEncodedLen
        + Into<Self::AccountId>;

    /// Indicator for when to end the session.
    type ShouldEndSession: ShouldEndSession<BlockNumberFor<Self>>;

    /// Something that can predict the next session rotation. This should typically come from
    /// the same logical unit that provides [`ShouldEndSession`].
    type NextSessionRotation: EstimateNextSessionRotation<BlockNumberFor<Self>>;

    /// Handler for managing new session.
    type SessionManager: SessionManager<Self::ValidatorId, Self::Keys>;

    /// Handler when a session has changed.
    type SessionHandler: SessionHandler<Self::ValidatorId>;

    /// The keys.
    type Keys: OpaqueKeys + Member + Parameter + MaybeSerializeDeserialize;
}
```

## Storage

The pallet maintains several storage items:

1. `ValidatorsAndKeys`: Stores the current set of validators and their associated session keys
2. `Validators`: Compatibility storage for Polkadot.js (only used when the `polkadot-js-compat` feature is enabled)
3. `CurrentIndex`: The index of the current session
4. `DisabledValidators`: Indices of validators that have been disabled in the current session

## API Specification

### Extrinsics

The Partner Chains Session pallet does not expose direct extrinsics. Session management is handled automatically through hooks and internal logic.

### Public Functions (API)

#### validators
Returns the current set of validators

```rust
fn validators() -> Vec<T::ValidatorId>
```

Returns:
- `Vec<T::ValidatorId>`: List of current validator IDs

#### validators_and_keys
Returns the current set of validators with their associated session keys

```rust
fn validators_and_keys() -> Vec<(T::ValidatorId, T::Keys)>
```

Returns:
- `Vec<(T::ValidatorId, T::Keys)>`: List of (validator_id, keys) pairs

#### current_index
Returns the current session index

```rust
fn current_index() -> SessionIndex
```

Returns:
- `SessionIndex`: The index of the current session

#### disabled_validators
Returns the list of indices of disabled validators

```rust
fn disabled_validators() -> Vec<u32>
```

Returns:
- `Vec<u32>`: Indices of validators that have been disabled

#### rotate_session
Rotates to a new session, registering a new validator set

```rust
fn rotate_session()
```

#### disable_index
Disables the validator at the specified index

```rust
fn disable_index(i: u32) -> bool
```

Parameters:
- `i`: The index of the validator to disable

Returns:
- `bool`: Whether the validator was successfully disabled (false if already disabled)

#### disable
Disables the validator with the specified validator ID

```rust
fn disable(c: &T::ValidatorId) -> bool
```

Parameters:
- `c`: The ID of the validator to disable

Returns:
- `bool`: Whether the validator was successfully disabled (false if not found or already disabled)

#### new_session
Prepares for a new session with the given index

```rust
fn new_session(index: SessionIndex) -> Option<Vec<(T::ValidatorId, T::Keys)>>
```

Parameters:
- `index`: The index of the new session

Returns:
- `Option<Vec<(T::ValidatorId, T::Keys)>>`: The new set of validators and keys, if available

#### new_session_genesis
Special handler for the genesis session

```rust
fn new_session_genesis(index: SessionIndex) -> Option<Vec<(T::ValidatorId, T::Keys)>>
```

Parameters:
- `index`: The index of the genesis session (typically 0)

Returns:
- `Option<Vec<(T::ValidatorId, T::Keys)>>`: The genesis set of validators and keys

### Inherent Data

The Partner Chains Session pallet does not use inherent data directly, but it does interact with other pallets that may provide session-related data through the inherent mechanism.

### Events

- `NewSession { session_index: SessionIndex }`: Emitted when a new session begins, providing the index of the new session

## Integration with Session Validator Management

The Partner Chains Session pallet is designed to work closely with the Session Validator Management pallet through the primitives in the session-manager crate. A typical integration pattern uses the `ValidatorManagementSessionManager` to bridge these two pallets:

```rust
impl pallet_partner_chains_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ShouldEndSession = ValidatorManagementSessionManager<Self>;
    type NextSessionRotation = ValidatorManagementSessionManager<Self>;
    type SessionManager = ValidatorManagementSessionManager<Self>;
    type SessionHandler = <Self as pallet_session_validator_management::Config>::SessionHandler;
    type Keys = <Self as pallet_session_validator_management::Config>::AuthorityKeys;
}
```

This setup ensures that:
1. Sessions end when a new validator committee is ready (typically at epoch boundaries)
2. The validator set is sourced from the Session Validator Management pallet
3. Session keys are properly propagated to the consensus mechanisms

## Compatibility with Substrate's Session Pallet

For runtimes that need to maintain compatibility with components expecting Substrate's original session pallet, the toolkit provides a compatibility helper through the `pallet_session_runtime_stub` crate:

```rust
impl_pallet_session_config!(Runtime);
```

This macro implements the standard `pallet_session::Config` trait for your runtime, delegating the core functionality to your `pallet_partner_chains_session::Config` implementation.

## Dependencies

- frame_system
- frame_support
- sp_runtime
- sp_staking