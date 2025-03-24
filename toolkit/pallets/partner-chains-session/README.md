# Partner Chains Session Pallet

## Overview

The Partner Chains Session pallet is a refined adaptation of Substrate's standard session pallet, customized to meet the requirements of partner chains while maintaining compatibility with core Substrate functionalities.

This pallet serves as the cornerstone for managing validator sets and their associated session keys within the partner chain network. It has been streamlined to eliminate unnecessary complexities while enhancing features that are critical for partner chains' consensus mechanisms.

The Partner Chains Session pallet operates on the principle of session-based validator rotation, where:

1. **Session Management**: A "session" represents a period during which a fixed set of validators is responsible for block production and finality. The pallet handles the orderly transition between sessions, ensuring consensus continuity.

2. **Key Management**: Validators require various cryptographic keys for different consensus-related functions. The pallet provides a framework for validators to register and update their session keys securely.

3. **Validator Rotation**: At the end of each session, the pallet can rotate to a new validator set, allowing the network to dynamically adjust its consensus participants based on various selection criteria.

4. **Validator Disabling**: The pallet includes mechanisms to disable validators during a session if they're found to be misbehaving, enhancing network security and resilience.

This pallet simplifies the approach to session management compared to the original session pallet. While the code still contains references to queued validators, they aren't actively used in the implemented logic, making the session transition more direct.

## Purpose

The Partner Chains Session pallet fulfills several critical functions within the partner chain ecosystem:

- Manages the lifecycle of validator sessions, including session creation, rotation, and termination
- Facilitates the orderly transition between validator sets without disrupting consensus
- Serves as the interface between validator selection logic and the consensus mechanism
- Maintains backward compatibility with Substrate components that expect the standard session pallet interface
- Supports the disabling of validators who misbehave

## Hooks

The Partner Chains Session pallet implements the following FRAME hooks to manage session transitions and validator set updates:

### on_initialize

The `on_initialize` hook is called at the beginning of each block's execution, before any extrinsics are processed. For the Partner Chains Session pallet, this hook handles session rotation:

```rust
fn on_initialize(n: BlockNumberFor<T>) -> Weight {
    if T::ShouldEndSession::should_end_session(n) {
        Self::rotate_session();
        T::BlockWeights::get().max_block
    } else {
        Weight::zero()
    }
}
```

**Key responsibilities:**

1. **Session Transition Check**: The hook queries the `ShouldEndSession` implementation to determine if the current session should end with this block.

2. **Session Rotation**: If a session should end, the hook orchestrates the full session rotation process:
   - Calls `on_before_session_ending` on session handlers to prepare for session end
   - Calls `end_session` on the session manager to finish the current session
   - Increments the session index
   - Queries the session manager for a new validator set via `new_session`
   - Rotates to the new validator set and clears any previously disabled validators
   - Calls `start_session` on the session manager to initiate the new session
   - Emits a `NewSession` event
   - Calls `on_new_session` on session handlers to notify them of the new validator set

3. **Weight Management**: The hook returns an appropriate weight based on whether a session rotation occurred:
   - Returns the maximum block weight if a session rotation happened
   - Returns zero weight if no rotation occurred

This hook is the core mechanism for session transitions in the partner chain system. It ensures that validator sets are updated at appropriate times and that all components relying on session information are properly notified of changes.

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

- **validators**: Returns the current set of validators
  ```rust
  pub fn validators() -> Vec<T::ValidatorId>
  ```

- **validators_and_keys**: Returns the current set of validators with their keys
  ```rust
  pub fn validators_and_keys() -> Vec<(T::ValidatorId, T::Keys)>
  ```

- **current_index**: Returns the current session index
  ```rust
  pub fn current_index() -> SessionIndex
  ```

- **disabled_validators**: Returns the list of disabled validators
  ```rust
  pub fn disabled_validators() -> Vec<u32>
  ```

- **rotate_session**: Moves to the next session and registers a new validator set
  ```rust
  pub fn rotate_session()
  ```

- **disable_index**: Disables the validator at the specified index
  ```rust
  pub fn disable_index(i: u32) -> bool
  ```

- **disable**: Disables the validator with the specified validator ID
  ```rust
  pub fn disable(c: &T::ValidatorId) -> bool
  ```

### Events

- `NewSession { session_index: SessionIndex }`: Emitted when a new session begins, providing the index of the new session

## Integration with Session Management

The Partner Chains Session pallet is designed to work with any component that implements the `SessionManager` trait:

```rust
pub trait SessionManager<ValidatorId, Keys> {
    fn new_session(new_index: SessionIndex) -> Option<Vec<(ValidatorId, Keys)>>;
    fn new_session_genesis(new_index: SessionIndex) -> Option<Vec<(ValidatorId, Keys)>>;
    fn end_session(end_index: SessionIndex);
    fn start_session(start_index: SessionIndex);
}
```

## Compatibility

For compatibility with different environments, the pallet includes feature flags:

1. `pallet-session-compat`: Enables compatibility components that help integrate with systems expecting the standard Substrate session pallet

## Dependencies

- frame_system
- frame_support
- sp_runtime
- sp_staking