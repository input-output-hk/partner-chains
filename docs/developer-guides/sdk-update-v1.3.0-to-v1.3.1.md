# Migration from v1.3.0 to v1.3.1

This guide present steps required to migrate you project from Partner Chains SDK v1.3.0 to v1.3.1.

## Update SDK dependency

In your cargo.toml files, update the `partner-chains` dependency from `v1.3.0` to `v1.3.1`.

## Update Runtime

In your Runtime crate find where instance of `RuntimeVersion` is created and increase the `spec_version`,
because runtime code update is required.

In your Rutime find `impl pallet_sidechain::Config` for your runtime,
and add following code to it:
```rust
type MainChainScripts = sp_session_validator_management::MainChainScripts;

fn set_main_chain_scripts(scripts: Self::MainChainScripts) {
	pallet_session_validator_management::MainChainScriptsConfiguration::<Runtime>::set(scripts);
}
```

If you have any Runtime test, is will be necessary to update mock as well:
```rust
type MainChainScripts = ();

fn set_main_chain_scripts(scripts: Self::MainChainScripts) {}
```

## Update the on-chain runtime code

Perform regular runtime code update procedure:
* update nodes to version build with Partner Chains SDK v1.3.1
* use `system/setCode` extrinsic to update runtime code build with Partner Chains SDK v1.3.1
