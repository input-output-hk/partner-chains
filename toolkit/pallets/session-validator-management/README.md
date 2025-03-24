# Session Validator Management Pallet

## Overview

The Session Validator Management pallet serves as an orchestration layer for managing validator committees within the partner chain ecosystem. It provides a comprehensive framework for determining which validators should be selected for block production and consensus duties during each epoch, as well as securely transitioning between committee sets.

At its core, this pallet addresses the critical challenge of validator selection and rotation in a decentralized manner:

1. **Committee Selection**: The pallet implements mechanisms to select a committee of validators for each upcoming epoch based on predefined criteria and selection algorithms. This selection can incorporate multiple factors including stake, performance history, and randomness to ensure fair and secure validator rotation.

2. **Authority Recognition**: Once selected, validators need to be properly recognized as authorities within the consensus system. The pallet manages the association between validator identities and their authority credentials, ensuring seamless integration with the consensus layer.

3. **Secure Transitions**: Perhaps most importantly, the pallet guarantees secure transitions between different validator sets. This is crucial for maintaining network liveness and preventing security vulnerabilities during committee rotations.

4. **Committee Planning**: The pallet ensures that committees are always planned at least one epoch in advance, providing predictability and allowing validators to prepare for their duties.

5. **Main Chain Integration**: For partner chains that coordinate with a main chain (like Cardano), the pallet maintains relevant configuration data about main chain scripts and addresses, establishing a secure link between the two chains' governance systems.

This pallet is designed with the complexity of cross-chain validator selection in mind. It allows for sophisticated selection algorithms and seamless integration with both main chain data sources and the partner chain's session management system.

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

This pallet's storage has changed compared to its legacy version. See the [migrations README](src/migrations/README.md) for more information.

## Primitives

The Session Validator Management pallet relies on primitives defined in the `toolkit/primitives/session-validator-management` crate.

## Hooks

The Session Validator Management pallet implements the following FRAME hooks to handle committee management, inherent processing, and validator selection:

### on_initialize

The `on_initialize` hook is called at the beginning of each block's execution, before any extrinsics are processed. For the Session Validator Management pallet, this hook primarily handles initializing the committee for the first block.

**Key responsibilities:**

1. **First Block Initialization**: The hook ensures that the genesis committee is properly set as the committee for the first block's epoch, allowing the handover phase to occur correctly.

2. **Weight Calculation**: The hook returns an appropriate weight based on the operations performed, ensuring proper accounting of computational resources.

### on_runtime_upgrade

The pallet also implements logic that runs during runtime upgrades:

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

    /// Maximum number of validators that can be in a committee
    #[pallet::constant]
    type MaxValidators: Get<u32>;
    
    /// Type representing validator ID - unique identifier of a validator among all validators
    type AuthorityId: Member
        + Parameter
        + MaybeSerializeDeserialize
        + MaxEncodedLen
        + Ord
        + Into<Self::AccountId>;
    
    /// All validator's keys, needed by various consensus or utility algorithms
    type AuthorityKeys: Parameter + Member + MaybeSerializeDeserialize + Ord + MaxEncodedLen;
    
    /// Authority selection input data for calculating committee
    type AuthoritySelectionInputs: Parameter;
    
    /// Type representing epoch number
    type ScEpochNumber: Parameter
        + MaxEncodedLen
        + Zero
        + Display
        + Add
        + One
        + Default
        + Ord
        + Copy
        + From<u64>
        + Into<u64>;
    
    /// Type representing committee member with all its Authority Keys
    type CommitteeMember: Parameter
        + Member
        + MaybeSerializeDeserialize
        + MaxEncodedLen
        + CommitteeMember<AuthorityId = Self::AuthorityId, AuthorityKeys = Self::AuthorityKeys>;

    /// Function to select authorities based on input data and epoch
    fn select_authorities(
        input: Self::AuthoritySelectionInputs,
        sidechain_epoch: Self::ScEpochNumber,
    ) -> Option<BoundedVec<Self::CommitteeMember, Self::MaxValidators>>;

    /// Runtime function that provides epoch number
    fn current_epoch_number() -> Self::ScEpochNumber;

    /// Weight functions needed for pallet_session_validator_management
    type WeightInfo: WeightInfo;
}
```

## Storage

The pallet maintains several storage items:

1. `CurrentCommittee`: Information about the current committee and its epoch
2. `NextCommittee`: Information about the next committee and its epoch
3. `MainChainScriptsConfiguration`: Configuration data for main chain scripts related to validator candidacy

## API Specification

### Extrinsics

- **set**: Sets the validators for a future epoch (primarily called through inherents)
- **set_main_chain_scripts**: Updates the mainchain scripts configuration (requires root origin)

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
Yes, when no next committee has been set for future epochs. The pallet requires inherent data to determine committee selection.

### Events

The pallet defines events, although note that the current implementation in lib.rs has an empty Event enum. In practice, implementations typically include events such as:

- `CommitteeRotated`: Emitted when committee is rotated to a new epoch
- `CommitteeSet`: Emitted when a committee is set for a future epoch
- `MainChainScriptsSet`: Emitted when the mainchain scripts configuration is updated

### Errors

The pallet defines the following errors:

- `InvalidEpoch`: The epoch is invalid for the operation
- `UnnecessarySetCall`: The set call is unnecessary (committee is already set for the epoch)

## Integration Example

A typical integration will include:

1. Defining a `CommitteeMember` type that implements the required trait:

```rust
#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Eq, Debug, MaxEncodedLen)]
pub struct CommitteeMember {
    pub authority_id: AccountId,
    pub authority_keys: SessionKeys,
}

impl sp_session_validator_management::CommitteeMember for CommitteeMember {
    type AuthorityId = AccountId;
    type AuthorityKeys = SessionKeys;
    
    fn authority_id(&self) -> Self::AuthorityId {
        self.authority_id.clone()
    }
    
    fn authority_keys(&self) -> Self::AuthorityKeys {
        self.authority_keys.clone()
    }
}
```

2. Implementing a validator selection function:

```rust
fn select_authorities(
    inputs: SelectionInputs,
    epoch: EpochNumber,
) -> Option<BoundedVec<CommitteeMember, MaxValidatorsConfig>> {
    // Selection logic to choose validators from candidates
    let candidates = process_selection_inputs(inputs);
    if candidates.is_empty() {
        return None;
    }
    
    let selected = select_based_on_criteria(&candidates, epoch);
    Some(BoundedVec::truncate_from(selected))
}
```

3. Configuring the pallet in the runtime:

```rust
impl pallet_session_validator_management::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxValidators = MaxValidators;
    type AuthorityId = AccountId;
    type AuthorityKeys = SessionKeys;
    type AuthoritySelectionInputs = SelectionInputs;
    type ScEpochNumber = EpochNumber;
    type CommitteeMember = CommitteeMember;
    
    fn select_authorities(
        input: Self::AuthoritySelectionInputs,
        sidechain_epoch: Self::ScEpochNumber,
    ) -> Option<BoundedVec<Self::CommitteeMember, Self::MaxValidators>> {
        // Selection implementation
    }
    
    fn current_epoch_number() -> Self::ScEpochNumber {
        Sidechain::current_epoch_number()
    }
    
    type WeightInfo = weights::WeightInfo<Runtime>;
}
```

4. Integrating with the session management system:

```rust
impl pallet_partner_chains_session::Config for Runtime {
    // Session pallet configuration
    type ValidatorId = AccountId;
    type ShouldEndSession = SidechainEpochManager;
    type NextSessionRotation = SidechainEpochManager;
    type SessionManager = ValidatorManagementSessionManager<Runtime>;
    type SessionHandler = (Aura, Grandpa);
    type Keys = SessionKeys;
    type WeightInfo = weights::WeightInfo<Runtime>;
}
```

## Dependencies

- frame_system
- frame_support
- sp_runtime
- sp_session_validator_management
- sidechain_domain (for main chain types)
