# Session Validator Management Pallet

## Overview

The Session Validator Management pallet serves as an orchestration layer for managing validator committees within the partner chain ecosystem. It provides a comprehensive framework for determining which validators should be selected for block production and consensus duties during each epoch, as well as securely transitioning between committee sets.

At its core, this pallet addresses the critical challenge of validator selection and rotation in a decentralized manner:

1. **Committee Selection**: The pallet implements mechanisms to select a committee of validators for each upcoming epoch based on predefined criteria and selection algorithms. This selection can incorporate multiple factors including stake, performance history, and randomness to ensure fair and secure validator rotation.

2. **Authority Recognition**: Once selected, validators need to be properly recognized as authorities within the consensus system. The pallet manages the association between validator identities and their authority credentials, ensuring seamless integration with the consensus layer.

3. **Secure Transitions**: Perhaps most importantly, the pallet guarantees secure transitions between different validator sets. This is crucial for maintaining network liveness and preventing security vulnerabilities during committee rotations.

4. **Committee Planning**: The pallet ensures that committees are always planned at least one epoch in advance, providing predictability and allowing validators to prepare for their duties.

5. **Main Chain Integration**: For partner chains that coordinate with a main chain (like Cardano), the pallet maintains relevant configuration data about main chain scripts and addresses, establishing a secure link between the two chains' governance systems.

This pallet is designed with the complexity of cross-chain validator selection in mind. It allows for sophisticated selection algorithms, inherent data-based committee proposals, and seamless integration with both main chain data sources and the partner chain's session management system.

The Session Validator Management pallet works in close coordination with the Partner Chains Session pallet, with a clear separation of concerns: this pallet focuses on who should be validators and when they should rotate, while the Session pallet manages the active validator set during each session.

## Purpose

This pallet provides a way to rotate session validators based on arbitrary inputs and selection algorithm. Its key purposes include:

- Securely managing the rotation of validator committees across epochs
- Implementing flexible validator selection algorithms
- Facilitating coordination between main chain inputs and partner chain validator selection
- Ensuring validator committees are always prepared in advance
- Maintaining configuration data for cross-chain validator candidacy mechanisms
- Providing runtime APIs for querying committee information

## Migrations

This pallet's storage has changed compared to its legacy version. See [src/migrations/README.md] for more information.

## Primitives

The Session Validator Management pallet relies on primitives defined in the `toolkit/primitives/session-validator-management` crate.

<CLAUDEMIND_THINKING>
I need to create a hooks section for the session-validator-management pallet README. This should explain the hooks used by the pallet, what they do, and their role in the pallet's functionality.
</CLAUDEMIND_THINKING>

Here's a hooks section that could be added to the session-validator-management pallet README:

## Hooks

The Session Validator Management pallet implements the following FRAME hooks to handle committee management, inherent processing, and validator selection:

### on_initialize

The `on_initialize` hook is called at the beginning of each block's execution, before any extrinsics are processed. For the Session Validator Management pallet, this hook handles inherent verification and committee transitions:

```rust
fn on_initialize(n: BlockNumberFor<T>) -> Weight {
    // Function implementation
}
```

**Key responsibilities:**

1. **Inherent Verification Setup**: The hook establishes the verification system for authority selection inherent data to ensure:
    - When no next committee has been set for future epochs, an inherent with authority selection inputs must be provided
    - The calculated committee from the inherent data matches what should be expected
    - Authority selection inputs hash matches if provided

2. **Committee Checks**: The hook examines the current committee state to determine if new validators need to be selected for future epochs.

3. **Weight Calculation**: The hook returns an appropriate weight based on the operations performed, ensuring proper accounting of computational resources.

### on_finalize

The `on_finalize` hook is called at the end of each block's execution, after all extrinsics have been processed. For the Session Validator Management pallet, this hook ensures that committee data is properly processed:

```rust
fn on_finalize(n: BlockNumberFor<T>) -> Weight {
    // Function implementation
}
```

**Key responsibilities:**

1. **Pending Committee Processing**: If authority selection inputs were provided via inherent data, the hook finalizes the committee selection process:
    - Calculates the committee based on the inputs and filtering/selection strategies
    - Stores the new committee for the appropriate future epoch
    - Emits the appropriate events to signal committee updates

2. **Cleanup**: The hook clears any temporary data that was needed only during block execution.

### on_runtime_upgrade

Although not used as frequently as the other hooks, the pallet also implements logic that runs during runtime upgrades:

```rust
fn on_runtime_upgrade() -> Weight {
    migrations::migrate::<T>()
}
```

**Key responsibilities:**

1. **Storage Migration**: Handles migration of storage formats between different versions of the pallet, ensuring data integrity across runtime upgrades.

## Configuration

The pallet uses the following configuration traits:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// Type representing committee member with all its Authority Keys
    type CommitteeMember: Parameter + Member + MaybeSerializeDeserialize + CommitteeMember<
        AuthorityId = Self::AuthorityId,
        AuthorityKeys = Self::AuthorityKeys,
    > + Clone;
    
    /// Type representing epoch number
    type ScEpochNumber: Parameter + Member + Default + Copy + PartialOrd + AtLeast32BitUnsigned +
        TypeInfo;
    
    /// Type representing validator ID - unique identifier of a validator among all validators
    type AuthorityId: Parameter + Member + MaybeSerializeDeserialize + Debug + Clone +
        FromStr + From<Self::CommitteeMember>;
    
    /// All validator's keys, needed by various consensus or utility algorithms
    type AuthorityKeys: Member + Parameter + MaybeSerializeDeserialize + From<Self::CommitteeMember>;
    
    /// Runtime function that provides epoch number
    fn current_epoch_number() -> Self::ScEpochNumber;
    
    /// Authority selection input data for calculating `NextAuthorities`
    type AuthoritySelectionInputs: Parameter + Member + Debug + codec::FullCodec;
    
    /// Type that can filter invalid candidates from authority selection inputs
    type FilterCandidates: FilterCandidates<
        Self::CommitteeMember,
        Self::AuthoritySelectionInputs,
        Self::ScEpochNumber,
    >;
    
    /// Type that can select authorities from filtered candidates
    type SelectAuthorities: SelectAuthorities<
        Self::CommitteeMember,
        Self::AuthoritySelectionInputs,
        Self::ScEpochNumber,
    >;
}
```

## Storage

The pallet maintains several storage items:

1. `CurrentCommittee`: Information about the current committee and its epoch
2. `NextCommittee`: Information about the next committee and its epoch
3. `MainChainScripts`: Configuration data for main chain scripts related to validator candidacy

## API Specification

### Extrinsics

- **set**: Sets the validators for a future epoch
- **set_main_chain_scripts**: Updates the mainchain scripts configuration

### Public Functions (API)

- **get_next_unset_epoch_number**: Returns the next epoch number for which validators haven't been set
- **get_current_authority**: Returns the authority at the given index
- **get_current_authority_round_robin**: Returns the authority using round-robin selection
- **current_committee_storage**: Returns the current committee info
- **next_committee_storage**: Returns the next committee info
- **next_committee**: Returns the next committee
- **calculate_committee**: Calculates committee for given inputs
- **rotate_committee_to_next_epoch**: Rotates committee to the next epoch
- **get_current_committee**: Returns the current committee and epoch
- **get_next_committee**: Returns the next committee and epoch
- **get_main_chain_scripts**: Returns the mainchain scripts configuration

### Inherent Data

#### Inherent Identifier
```rust
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"/ariadne";
```

#### Data Type
`T::AuthoritySelectionInputs` - Input data for validator selection algorithm

#### Inherent Required
Yes, when no next committee has been set for future epochs. The pallet verifies this inherent data to ensure valid committee selection.

### Events

- `CommitteeRotated(T::ScEpochNumber)`: Emitted when committee is rotated to a new epoch
- `CommitteeSet(T::ScEpochNumber)`: Emitted when a committee is set for a future epoch
- `MainChainScriptsSet`: Emitted when the mainchain scripts configuration is updated

### Errors

- `EpochMustBeGreaterThanCurrentCommitteeEpoch`: The epoch for which committee is being set must be greater than the current committee epoch
- `EpochMustBeGreaterThanOrEqualToNextUnsetEpoch`: The epoch for which committee is being set must be greater than or equal to the next unset epoch
- `NotAllValidators`: The committee must include all validators
- `CommitteeAlreadySetForEpoch`: Committee is already set for the specified epoch
- `NextCommitteeNotSet`: Next committee has not been set
- `AuthoritySelectionFailed`: Failed to select authorities
- `InputHashMismatch`: Input hash does not match calculated hash

## Integration Example

A typical integration will include:

1. Implementing the necessary selection logic:

```rust
pub struct FilterInvalidCandidates;
impl FilterCandidates<CommitteeMember, SelectionInputs, EpochNumber> for FilterInvalidCandidates {
    fn filter_candidates(
        candidates: &[CommitteeMember],
        _inputs: &SelectionInputs,
        _epoch: EpochNumber,
    ) -> Vec<CommitteeMember> {
        // Filter logic to remove invalid candidates
        candidates.to_vec()
    }
}

pub struct RandomAuthoritiesSelector;
impl SelectAuthorities<CommitteeMember, SelectionInputs, EpochNumber> for RandomAuthoritiesSelector {
    fn select_authorities(
        candidates: &[CommitteeMember],
        inputs: &SelectionInputs,
        epoch: EpochNumber,
    ) -> Option<Vec<CommitteeMember>> {
        // Selection logic to choose validators from candidates
        Some(weighted_random_selection(candidates, inputs, epoch))
    }
}
```

2. Configuring the pallet in the runtime:

```rust
impl pallet_session_validator_management::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type CommitteeMember = CommitteeMember;
    type ScEpochNumber = EpochNumber;
    type AuthorityId = AccountId;
    type AuthorityKeys = SessionKeys;
    
    fn current_epoch_number() -> Self::ScEpochNumber {
        Sidechain::current_epoch_number()
    }
    
    type AuthoritySelectionInputs = SelectionInputs;
    type FilterCandidates = FilterInvalidCandidates;
    type SelectAuthorities = RandomAuthoritiesSelector;
}
```

3. Integrating with the session management system:

```rust
impl pallet_partner_chains_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ShouldEndSession = ValidatorManagementSessionManager<Self>;
    type NextSessionRotation = ValidatorManagementSessionManager<Self>;
    type SessionManager = ValidatorManagementSessionManager<Self>;
    type SessionHandler = (Aura, Grandpa);
    type Keys = SessionKeys;
}
```

## Dependencies

- frame_system
- frame_support
- sp_runtime
- sp_staking
- sidechain_domain (for main chain types)
