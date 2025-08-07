# Migration from v1.7.1 to v1.8.0

## Generic keys

The biggest change in the Partner Chains Toolkit v1.8.0 is the support of generic session keys.
One implication of having the keys generic and not hardcoded is that the toolkit users have to inject some behavior to the toolkit.
This dependency injection is required by `partner-chains-cli` create (`wizards` subcommand), and it is done by implementing the `PartnerChainRuntime` trait.
Following items have to be defined:
* `type Keys: MaybeFromCandidateKeys` - should be the type of the session keys used in the Runtime. This type is used for the chain-spec creation, to insert initial committee and validators keys into genesis.
* `fn key_definitions() -> Vec<KeyDefinition<'static>>` - should define textual information about `Keys`. This function is used in the user interface and for interactions with the substrate CLI keystore commands.
* `fn create_chain_spec(config: &CreateChainSpecConfig<Self::Keys>) -> serde_json::Value` - should return a JSON object of the chain-spec. This function is used by the `create-chain-spec` subcommand. The toolkit no longer makes assumptions about `build-spec` behavior when no `--chain` parameter is supplied. The toolkit provides functions from `CreateChainSpecConfig` to `GenesisConfig`, for each of the pallets, to be used when implementing `create_chain_spec`.

Changes required:
1. Add `impl authority_selection_inherents::MaybeFromCandidateKeys for SessionKeys {}` for your runtime SessionKeys type
1. `select_authorities` function now requires explicit type annotations, please provide them in your runtime implementation, also remove wrapping of the result into `BoundedVec` - function returns this type now
1. Add dependency on `partner-chains-cli` in the node (final executable) crate
1. Implement `PartnerChainRuntime` for your chain and pass it to the code that wires in partner-chains-commands
1. Remove `impl RuntimeTypeWrapper`

## Export changes in `authority-selection-inherents` crate

* `authority-selection-inherents` crate now exports types from the top level, please adjust `use` clauses.

## `pallet-address-associations` - security fixes

This pallet is now protected against space attacks. Runtime implementation has to be updated.

* Please define cost of association:
```rust
parameter_types! {
	/// Amount of tokens to burn when making irreversible, forever association
	pub const AddressAssociationBurnAmount: Balance = 1_000_000;
}
```
* Set the cost and currency in the pallet config:
```rust
type Currency = Balances;
type BurnAmount = AddressAssociationBurnAmount;
```

## `block-producer-metadata` - security fixes

The pallet is now protected against space, front running and replay attacks
Required changes are:
* update `type RuntimeHoldReason = RuntimeHoldReason;` in `pallet-balances` config implementations
* define the metadata deposit amount for space attack prevention:
```rust
parameter_types! {
	/// Amount of tokens to hold when upserting block producer metadata.
	pub const MetadataHoldAmount: Balance = 1_000_000;
}
```
* set the hold amount, currency and possible reason types and the current time getter in the `pallet-block-producer-metadata` implementation config implementation:
```rust
type Currency = Balances;
type HoldAmount = MetadataHoldAmount;
type RuntimeHoldReason = RuntimeHoldReason;

fn current_time() -> u64 {
	pallet_timestamp::Now::<Runtime>::get() / 1000
}
```
