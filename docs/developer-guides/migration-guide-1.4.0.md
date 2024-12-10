# v1.4.0 Migration guide

## Context

This guide describes the process of migrating from Partner Chains SDK v1.3.0 to v1.4.0 for an already
established chain, avoiding a chain reset.

The biggest change in version v1.4.0 which requires special handling is the update to smart contracts
version v7.0.1, which:
- introduces a new governance mechanism which needs to be set up
- removes "sidechain params" as part of the definition of a Partner Chain, replacing them with the
genesis utxo (which is the utxo burned when establishing a governance)

## Migration Steps

### Prerequisites

This guide assumes the following:
1. An already running Partner Chain is present running nodes and runtime of version v1.3.0
and observing configuration and registrations created using smart contracts of version v6.2.2.
2. The Partner Chain's original chain spec file is available.

### Runtime upgrade to v1.3.1

This patch version extends the Sidechain pallet with a new extrinsic `upgrade_and_set_addresses`
which allows the on-chain governance to atomically upgrade the runtime and set the genesis utxo and new main chain
scripts to observe.

1. Update Partner Chains SDK dependencies in your `Cargo.toml` to `v1.3.1`
2. Add the following configuration to your runtime configuration:
```rust
impl pallet_sidechain::Config for Runtime {
    // ... other parameters
	type MainChainScripts = sp_session_validator_management::MainChainScripts;

	fn set_main_chain_scripts(scripts: Self::MainChainScripts) {
		pallet_session_validator_management::MainChainScriptsConfiguration::<Runtime>::set(scripts);
	}
}
```
This will allow the `upgrade_and_set_addresses` extrinsic to update configuration of the SessionValidatorManagement
pallet together with the Runtime code.
3. Increment the `spec_version` in your runtime configuration.
4. Build the new Runtime WASM (you can do it by running `cargo build --release`)
5. Upgrade the runtime of the chain using the newly built WASM (it can be found in `$CARGO_TARGET_DIR/release/wbuild/<runtime name>/<runtime name>.compressed.wasm`) and the
extrinsic `system/setCode`.

After these steps the chain should be running the runtime using Partner Chains SDK v1.3.1. This version is
fully backwards-compatible and is a preparatory step towards v1.4.0

### Release new version using PC-SDK v1.4.0

This version of the node is backwards-compatible with the runtime in versions 1.3.x. and is needed to support
the runtime in version v1.4.0.

1. Update all Partner Chains SDK dependencies in your `Cargo.toml` file to `v1.4.0`
2. Modify your runtime and node code to be compatible with PC-SDK version v1.4.0. Consult the release notes for v1.4.0
for required changes.
3. Release a new version of your node. This step depends on your release process.
4. Upgrade nodes running the chain to the newly released version. This step depends on your deployment process.
Irrespective of the deployment details, the nodes should be run with the same keystores as previously.

After this step, the network should be composed of nodes v1.4.0 but the runtime would remain at v1.3.1, and be ready for the matching
runtime upgrade to v1.4.0.

### New main chain state setup

Because of the changes to the smart contracts and their on-chain data, a completely new set
of main chain data needs to be set up on Cardano.

**Important:**
The commands in this section should either be run in the same directory used for setting up the previous Partner Chain
(containing the base data directory containing the keystore and the `partner-chains-cli-keys.json` file),
or in a new directory that has been prepared by running the `generate-keys` commands.

1. Pick the initial governance authority.
This can be a newly generated or already existing main chain key set (including the current governance authority of the Partner Chain being upgraded)
controlled by the Partner Chain authority.
The address associated with the keys should have enough ADA to cover transaction costs (10 or more ADA is advised).
2. Run the `prepare-configuration` command of `partner-chains-cli` (v1.4.0) in a fresh directory.
This step will involve selecting the _genesis UTXO_ to be spent intializing the governance mechanism.
Save the `partner-chains-cli-chain-config.json` file produced by this step.
3. Run the `create-chain-spec` command of `partner-chains-cli`. Save the `chain-spec.json` file produced by this step.
4. Add the permissioned candidates in the `partner-chains-cli-chain-config.json` file. These can be copied from the chain config file
used when setting up the Partner Chain previously, or obtained by querying the `sidechain_getAriadneParameters` jsonRPC method:
```sh
curl "<PC node>" -X POST -H "Content-Type: application/json" -d '{"jsonrpc": "2.0", "id":0, "method":"sidechain_getAriadneParameters","params":[<epoch>] }' | jq '.result.permissionedCandidates'
```
5. Run the `setup-main-chain-state` command of `partner-chains-cli`, setting up the D-param and permissioned candidates.

After these steps, the new Partner Chain will be initialized on Cardano.

### Re-register candidates

At this point all SPOs willing to continue participating in the chain must re-register themselves with the newly created Partner Chain on Cardano.

1. Distribute the `partner-chains-cli-chain-config.json` and `chain-spec.json` files produced in the previous section to the SPOs.
2. Each individual SPO must use it to go through the registration process
by running the `register1`, `register2`, `register3` commands of `partner-chains-cli` (v1.4.0).

After this step the SPO should be ready to be included in post-migration committee selection.

*Important:*
* the `chain-spec.json` file is only used for registrations and should **not** be used to run the nodes.
The SPOs should discard it after this section.
* For the register commands to be run correctly, the `partner-chains-public-keys.json` file needs to be present
in the run directory and the base path configured in `partner-chains-cli-resources.json` should point to a valid
directory containing the keystore with the private keys.
If the SPO used `partner-chains-cli` for the previous registration, they should re-use the keys generated then.
New SPOs should run the `generate-keys` command instead.

### Upgrade the runtime to v1.4.0

This step will switch the Partner Chain from observing the old on-chain Cardano data to the new one created in the
previous steps. These steps should be performed after *at least two* Cardano epochs after SPO re-registrations were performed.


1. get new addresses and genesis utxo from the `partner-chains-cli-chain-config.json` file, eg.:
```json
{
  "cardano_addresses": {
    "committee_candidates_address": "addr_test1wzhgt0xew2pen9degvtmepjwcezwwjtv8g5uz04259etlec9nzg7q",
    "d_parameter_policy_id": "0f50a93aec22303d900f405e0efecd6ca88e2045715663e102498260",
    "permissioned_candidates_policy_id": "9a124a13147573e28853c99b14cf3afc4193e26250e6c1949723379b",
    ...
  },
  "chain_parameters": {
    "genesis_utxo": "0d7f8f42a7394af289bf3e1da6c113702d76d50e24fecfd810328db9f908dd74#1"
  },
  ...
}
```
2. Perform atomic upgrade of runtime and observed addresses by invoking the `sidechain/upgradeAndSetAddresses`
extrinsic using `sudo` or other Substrate governance mechanism.
This requires providing the following values to the extrinsic:
    - runtime WASM code (this code should come from the same release as the v1.4.0 node)
    - genesis UTXO (`chain_parameters.genesis_utxo`)
    - committee candidates validator address (`cardano_addresses.committee_candidates_address`)
    - D-parameter policy ID (`cardano_addresses.d_parameter_policy_id`)
    - permissioned candidates policy ID (`cardano_addresses.permissioned_candidates_policy_id`)
**warning**: the values provided to the extrinsic _must_ be correct. Using incorrect values will result in the chain
stalling if the extrinsic is executed. A good way to make sure the values are correct is to use them to run a local
chain first with the node and runtime at version v1.4.0.
After this extrinsic passes, the Partner Chain should start producing blocks and selecting subsequent committees based on the newly set up Cardano state.
