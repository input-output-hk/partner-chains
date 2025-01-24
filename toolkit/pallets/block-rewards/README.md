# Block Rewards Pallet

This crate implements the runtime component of the block rewards whose task is to track
allocation of rewards for block production and allow processing of this information for
reward payouts.

This crate works together with the `sp-block-rewards` crate to implement this functionality.

## Usage

*Note*: Some type names used below refer to type fields in the `Config` trait of the pallet.
For clarity, these are prefixed with `T::` similarly to how they're used in the code (where
`T` stands for the runtime generic type).

### Block beneficiary

Rewards for block production are credited to a `beneficiary`, identified by a `T::BeneficiaryId`,
which is a chain-specific type that needs to be provided by the developer.
When each block is produced, the `beneficiary` is determined by the inherent data provided by the
`BlockBeneficiaryInherentProvider`. This inherent data provider can be construed in an arbitrary
way, but `BlockBeneficiaryInherentProvider::from_env` helper method on the type is provided to fetch
the ID from the node's local environment.

The value of the beneficiary ID is chosen at the sole discretion of each particular
block producing node's operator and is not subject to any checks beyond simple format validation
(in particular, it is not checked whether the node operator controls any keys related to the ID).

### Block value

Rewards credited for each block's production can be calculated arbitrarily, by providing the
`T::BlockRewardPoints` type representing the rewards and `T::GetBlockRewardPoints: GetBlockRewardPoints` type
implementing the logic determining current block value.

For simple cases, type `SimpleBlockCount: GetBlockRewardPoints` is provided that assigns a constant
value of 1 to every block, for any `T::BlockRewardPoints` implementing the `One` trait.

### Pallet operation

The pallet ingests the inherent data and keeps a tally of beneficiaries and their accumulated
rewards in its on-chain storage. The rewards are only credited for blocks whose production
was successfuly finalized.

### Processing the rewards

To make the reward information available for processing, the pallet exposes the `get_rewards_and_clear`
function, which returns all accumulated reward data and resets the pallet storage. This function
should be called from the chain-specific runtime code that implements the payout mechanism, and it is
the calling logic's responsibility to process all returned reward entries.

`get_rewards_and_clear` can be called with any frequency, depending on how often reward payouts are to
be performed. For Partner Chains using `pallet-sidechain`, it's convenient to put the payouts logic
in the `OnNewEpoch` handler defined in the Sidechain pallet's `Config`.
