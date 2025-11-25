# Migrating a Partner Chain to use Beefy

## Introduction

This document describes step by step how to extend an already running Partner Chain
with Beefy. The migration path described maintains backward compatibility, avoids a
chain reset and allows historical blocks to be imported by new nodes post-migration.

## Warnings

1. Remember that runtime upgrades are inherently risky and can brick a chain if done
incorrectly. **Always test any runtime upgrade before applying it to a live chain.** This
can be done using [try-runtime](https://github.com/paritytech/try-runtime-cli) for
quickly verifying only the runtime and storage updates, or using a locally run chain,
ideally one cloned from the target live chain. It is advised to do both.
2. For security, Partner Chain committee selection requires that a committee is chosen
and produces at least one block for each Partner Chain epoch. This means that a chain
will **stall indefinitely** if at some point the valid candidate pool for some epoch is
empty.
3. Any on-chain updates to the authority selection inputs on Cardano (SPO registrations,
D-Param and permissioned candidate list) **only become effective after two full Cardano
epochs have passed**. If the updates are done in preparation for subsequent changes to the
Partner Chain that would invalidate the previous on-chain values, at least this much time
should be observed before applying these changes.

## Context and considerations

Beefy is a bridge-oriented protocol that provides state inclusion proofs for cross-chain
communication. It is not a finality gadget by itself and builds upon existing finality
mechanisms such as Grandpa.

Node components of Beefy are capable of detecting the pallet in the runtime and will stay
inactive until it is present and the Beefy genesis block is set. This means that nodes can
be prepared to support Beefy before the migration and still support historical blocks as
needed.

Beefy introduces its own set of authority keys that are used by committee members to sign
finality proofs each round. These keys are managed by `pallet-session` together with other
session keys used by the chain. The main concern when adding Beefy to a running chain is
migrating the existing key storages and ensuring that block producer's Beefy keys are set
before Beefy starts operating.

The Partner Chain toolkit has been prepared to support migrating to Beefy and similar
functionalities that require introduction of new authority keys, with the only requirement
being that the feature's implementation makes it possible to postpone its activation for
some time after being added to the runtime.

## Migration steps outline

In short, the migration plan is as follows:

0. Update candidate data to include Beefy keys by updating the permissioned candidate list
   and instructing SPOs to re-register themselves
1. Add Beefy offchain services to the node code and upgrade nodes in the network, including
   Beefy keys in their keystores.
2. Add Beefy to the runtime, together with all supporting pallets
3. Extend the session keys type to include Beefy keys
4. Schedule session keys type migration
5. Perform runtime upgrade
6. Wait for a new session, which will update authority keys
7. Set new genesis block in Beefy to activate the pallet


## Migration steps details

### Updating candidate data

When in
the next steps the runtime is upgraded and Beefy keys are added to the session keys type,
any candidate missing them in their candidate data on Cardano will be rejected by the commitee
selection algorithm.

To prepare for that, the Partner Chains toolkit allows candidate data stored on Cardano to include
additional keys to those that are currently required. Those keys will become visible to the Partner
Chain after two Cardano epochs have passed.

In this preparatory step, both the governance authority of the Partner Chain and individual Cardano
SPOs should update data on-chain to include Beefy keys.

First, every SPO and permissioned node operator needs to generate their Beefy key and insert it into
their keystore. The key can be generated using the command:
```shell
$ pc-node key generate --scheme ecdsa
```
and the generated secret phrase can be inserted to the keystore using:
```shell
$ pc-node key insert --scheme ecdsa --key-type beef --suri "<secret phrase>" --keystore-path "<keystore path>"
```

The governance authority should prepare the permissioned candidate list in CSV format in the following format:
```csv
<cross-chain skey hex>,aura:<aura skey hex>,gran:<grandpa skey hex>,beef:<beefy skey hex>
<cross-chain skey hex>,aura:<aura skey hex>,gran:<grandpa skey hex>,beef:<beefy skey hex>
...
```
and put it on-chain using the following command:
```shell
$ pc-node smart-contracts upsert-permissioned-candidates
    --permissioned-candidates-file <permissioned candidate file>
    --payment-key-file <governance skey>
    --genesis-utxo <genesis utxo>
```

The SPOs should re-register their node using the wizard (see `docs/user-guides/registered.md` for instructions)
or by manually creating relevant signatures using:
```shell
$ pc-node registration-signatures \
    --genesis-utxo <genesis utxo> \
    --mainchain-signing-key <Cardano skey hex> \
    --sidechain-signing-key <cross-chain skey hex> \
    --registration-utxo <registration utxo> # UTXO spendable from the candidate's wallet
```
and then submitting them on-chain using:
```shell
$ pc-node smart-contracts register \
    --genesis-utxo <genesis utxo> \
    --spo-public-key <Cardano pkey> \
    --spo-signature <Cardano signature> \
    --partner-chain-public-keys <cross-chain pkey>,aura:<aura pkey>,gran:<grandpa pkey>,beef:<beefy pkey> \
    --sidechain-signature <cross-chain signature> \
    --registration-utxo <registration utxo> \ # same as in previous command
    --payment-key-file <Cardano skey file>
```

This process should be coordinated between the governance authority and the SPOs to allow enough time for everyone
to go through the steps before the runtime upgrade.

Note that the nodes in the network can be freely upgraded while the re-registration period is ongoing. 

### Upgrading network nodes

This step can be performed during or even before the re-registrations.

The node code of the Partner Chain should be updated to include Beefy services and then rolled out to the network.

Adding Beefy components to the node is quite involved, so the reader is encouraged to reference the demo node
implementation for an example of how to do it, most importantly `demo/node/src/service.rs` for
offchain services and `demo/node/src/rpc.rs` for RPC services.
Keep in mind that due to Substrate using strong typing where it's possible, some changes to the runtime code will
be required at this point.

After the node code has been updated, new node binary should be distributed to the node operators and an upgrade
period should be coordinated with enough time for the majority to upgrade their nodes. 

### Adding Beefy to the runtime

This step can be performed during re-registrations and together with the node code modifications in the previous step.

The Beefy pallet should be added to the runtime and configured, together with other pallets that support its operation:
- `pallet_beefy`
- `pallet_mmr`
- `pallet_beefy_mmr`

Refer to the demo runtime at `demo/runtime/src` for an example of how to integrate these
pallets to your runtime.

### Adding Beefy keys to session keys type

Adding Beefy to session keys requires just adding another field to the `impl_opaque_keys` macro.
For a chain that uses Aura and Grandpa for consensus, it would look like this:
```rust
impl_opaque_keys! {
    #[derive(MaxEncodedLen, PartialOrd, Ord)]
    pub struct SessionKeys {
        pub aura: Aura,
        pub grandpa: Grandpa,
        pub beefy: Beefy,
    }
}
```

Keep in mind that modifying the session keys type means that from the next runtime upgrade, the added
key type becomes mandatory and any candidate whose registration data doesn't include them will be
considered invalid.

### Scheduling session keys migration

Any modification of the session keys type requires a storage migration that will update the values stored
in `pallet-session-validator-management` and `pallet-session`.

First, the original session keys type should be preserved under a different name. Eg:

```rust
impl_opaque_keys! {
    #[derive(MaxEncodedLen, PartialOrd, Ord)]
    pub struct LegacySessionKeys {
        pub aura: Aura,
        pub grandpa: Grandpa,
    }
}
```

Next, a conversion from the old type to the new type should be defined by implementing the `UpgradeAuthorityKeys`
trait. When a new field is added to the type, a valid default value should be used, eg.:

```rust
impl UpgradeAuthorityKeys<SessionKeys> for LegacySessionKeys {
    fn upgrade(self) -> SessionKeys {
        SessionKeys {
            aura: self.aura,
            grandpa: self.grandpa,
            beefy: ecdsa::Public::default().into(),
        }
    }
}
```

Finally, a key migration should be added to the runtime's migration list, using `AuthorityKeysMigration`
provided by `pallet-session-validator-management`, eg:

```rust
pub type Migrations = (
	AuthorityKeysMigration<Runtime, opaque::LegacySessionKeys, 0, 1>,
);
```

The migration is versioned to prevent re-running it in case it is not removed from the runtime before
subsequent upgrades. The current version of the session keys used by the runtime is tracked by
`pallet-session-validator-management` and updated as part of the migration. In the example, the migration
is defined as going from session keys version 0 to version 1, which will be the case for chains that
never migrated their keys using this mechanism before.

### Upgrading the runtime
 
**Important:** This step should only be performed once at least one committee member candidate's registration
data containing Beefy keys becomes available for observation. This happens after *two full Cardano epochs*
have passed after it was updated.

After the runtime code has been updated, including the session keys, the on-chain runtime should be updated
by calling `system/setCode` extrinsic through the governance mechanism used by the Partner Chain, eg. `sudo`
or `democracy` pallet.

### Wait for session change

Note that since a default value is used by the storage migration for the newly added keys, they will not be
usable right away after the runtime upgrade. This fact makes it crucial that Beefy (or any similar feature
being added) must not become active immediately after runtime upgrade. Luckyly, in this case Beefy pallet
starts in a dormat state until it is activated.

Real Beefy keys of the committee members will be sourced from Cardano and registered in `pallet-session`
after one or two session rotations after the runtime upgrade, depending on the exact timing. The keys
should still be manually verified to have been updated by reading the `currentCommittee` storage of the
session management pallet.

### Activating Beefy

Once the sessions rotate and the current committee has its Beefy keys correctly updated, the Beefy pallet
can be activated by invoking the `beefy/setNewGenesis` extrinsic using the governance method employed
by the Partner Chain in question. This extrinsic accepts an offset of blocks after which the Beefy pallet
will become active. Once it happens, the block producers will start participating in Beefy consensus rounds.
At this point, it can be verified that Beefy is working correctly by checking logs for lines reporting
successful rounds, like the following one:
```
2025-11-27 12:42:02 🥩 Concluded mandatory round #1
```
