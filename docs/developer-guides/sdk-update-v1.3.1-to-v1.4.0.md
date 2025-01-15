# Migration from v1.3.1 to v1.4.0

This guide present steps required to migrate your project from partner-chains v1.3.0 to v1.3.1.

The most notable change in v1.4.0 is removal of the SidechainParams.
Sections about Node and Runtime update will explain how to deal with this removal.

## Update SDK dependency

Most probably your code depended on `chain-params` crate from `partner-chains`.
Please remove this dependency from your `Cargo.toml` files.

In your cargo.toml files, update the `partner-chains` dependency from `v1.3.1` to `v1.4.0`.

### Repository reorganization

Between v1.3 and v1.4 there was a major reorganization of the repository.

Reference runtime implementation crate `sidechain-runtime` has been moved to `node/runtime`,
and reference node implementation crate `partner-chains-node` has been moved to `node/node`.

All other crates have been moved to `toolkit` directory.

Crates names have not been changed.

## Update Runtime crate

In your Runtime crate find where instance of `RuntimeVersion` is created and increase the `spec_version`,
because runtime code update is required.

In you Runtime find `impl pallet_session_validator_management::Config` for your runtime type,
and replace
```rust
select_authorities(Sidechain::sidechain_params(), input, sidechain_epoch)
```
with
```rust
select_authorities(Sidechain::genesis_utxo(), input, sidechain_epoch)
```
because SidechainParameters were removed and the genesis UTXO is now the sole idenfier of the chain.

Find `impl pallet_sidechain::Config` and remove following code from it:
```rust
type SidechainParams = chain_params::SidechainParams;
```

In `impl_runtime_apis!` please replace:
```rust
impl sp_sidechain::GetSidechainParams<Block, SidechainParams> for Runtime {
	fn sidechain_params() -> SidechainParams {
		Sidechain::sidechain_params()
	}
}
```
with
```rust
impl sp_sidechain::GetGenesisUtxo<Block> for Runtime {
	fn genesis_utxo() -> UtxoId {
		Sidechain::genesis_utxo()
	}
}
```

In `impl authority_selection_inherents::filter_invalid_candidates::CandidateValidationApi` please replace:
```rust
authority_selection_inherents::filter_invalid_candidates::validate_registration_data(mainchain_pub_key, registration_data, &Sidechain::sidechain_params()).err()
```
with
```rust
authority_selection_inherents::filter_invalid_candidates::validate_registration_data(mainchain_pub_key, registration_data, Sidechain::genesis_utxo()).err()
```

If your have any Runtime test, is will be necessary to them by replacing mock SidechainParams with a mock UtxoId.

## Update Node crate

Replace `sp_sidechain::GetSidechainParams<Block, SidechainParams>` with `sp_sidechain::GetSidechainParams<Block>` in RPC implementation.

In the code used to build a chain spec, please update `sidechain: SidechainConfig` creation.
The `param` field is replaced with `genesis_utxo`.
To keep reading this value from environment, please use: `sp_sidechain::read_genesis_utxo_from_env_with_defaults`.

Example:
```rust
sidechain: SidechainConfig {
	genesis_utxo: sp_sidechain::read_genesis_utxo_from_env_with_defaults()?,
	..Default::default()
}
```

If your code has any tests in the node crate, please replace creation of mock `SidechainParams` with a mock `UtxoId`.
