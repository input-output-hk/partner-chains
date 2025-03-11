# Partner Chains governance on Cardano

This document explains how to initialize and use the Governance System of Partner Chains Cardano Smart Contracts.

## Recap

Each Partner Chain is identified by a unique identifier called *genesis UTXO*.
For the same version of Smart Contracts, using a different *genesis UTXO* will result in different Cardano addresses and policy ids of these contracts.

## Governance system capabilities

The Governance System sets and updates the key required to sign the following transactions:

* management of *D-Parameter*
* management of *Permissioned Candidates List*
* management of *Rewards Reserve Mechanism* lifecycle: initialization, creation, and handover
* update of the Governance System itself

The Governance System has to be initialized before performing any of these operations.

In the current shape of the Governance System, it allows to set or update a single authorization key.
In other words, there is one key that is required sign all of the transactions above, but it can be changed to another key.
See [Technical details](#technical-details) for more information regarding this limitation.

## Governance System Initialization

The initialization of the Governance System for a given *genesis UTXO* spends that UTXO.
This means it can be done only once for a given *genesis UTXO*.

Other than spending *genesis UTXO*, the initialization records on Cardano, that given private key is required to sign operations on smart contracts that use the Governance System.
This private key is identified by its public key hash.

### Usage

In version v1.5 the command to initialize Governance System is available in the Partner Chains compatible node executable:

```bash
./partner-chains-node smart-contracts governance init \
	--genesis-utxo <GENESIS_UTXO> \
	--ogmios-url <OGMIOS_URL> \
	--payment-key-file <PAYMENT_KEY_FILE> \
	--governance-authority <GOVERNANCE_AUTHORITY>
```

* `<GENESIS_UTXO>`: The genesis UTXO of the Partner Chain, it will be spend by the transaction, and it has to be present in the wallet belonging to the signing key.
* `<OGMIOS_URL>`: The URL of the Ogmios service connected to the Cardano node, it is optional and defaults to `ws://localhost:1337`.
* `<PAYMENT_KEY_FILE>`: Cardano Shelley Payment Signing Key file (normal or extended).
* `<GOVERNANCE_AUTHORITY>`: Hex encoded blake2b-224 hash of public key related to private key that will be required to sign governance operations.

Governance System initialization also takes place in the *prepare-configuration* wizard. See [Chain builder (initial chain governance)](./../chain-builder.md).

In version v1.4 this functionality is available in the smart contracts CLI application `pc-contracts-cli init-goverance`.

## Governance System update

Enables update of the key required to sign operations on smart contracts that use the Governance System.
Following its execution, the new key is required to sign operations, the old key is no longer valid for this purpose.

### Usage

In version v1.5 command to initialize governance is available in the Partner Chains compatible node executable:
```
./partner-chains-node smart-contracts governance update \
	--genesis-utxo <GENESIS_UTXO> \
	--ogmios-url <OGMIOS_URL> \
	--payment-key-file <PAYMENT_KEY_FILE> \
	--new-governance-authority <NEW_GOVERNANCE_AUTHORITY>
```

* `<GENESIS_UTXO>`: The genesis UTXO of the Partner Chain. Same as the one used for `governance init`.
* `<OGMIOS_URL>`: The URL of the Ogmios service connected to the Cardano node, it is optional and defaults to `ws://localhost:1337`.
* `<PAYMENT_KEY_FILE>`: Cardano Shelley Payment Signing Key file (normal or extended) of the current governance authority (ie. hash of its public key should equal current governance authority hey hash).
* `<NEW_GOVERNANCE_AUTHORITY>`: Hex encoded blake2b-224 hash of public key related to private key that will be required to sign governance operations following this operation.

In version v1.4 this functionality is available in the smart contracts CLI application `pc-contracts-cli update-governance`.

## Technical details

The Governance System was designed to be quite flexible. To achive this it keeps a single UTXO at the address of Governance Validator.
This UTXO has a complete script attached (*authorization script*).
The Governance system passes transactions through the *authorization script*, to check if the transaction meets conditions set by this *authorization script*.

Therefore, the current limitation of a single governance authority key is not an inherent limitation of the smart contract logic of the Governance System,
but merely stems from the lack of user interface and transaction building logic that would allow the handling of multiple keys.

This means that some other UI and transaction building logic (so-called offchain code) could be implemented,
to build transactions that have properties required by the used *authorization script*.

### Links

[Rewards reserve mechanism management](./../../developer-guides/native-token-reserve-management.md)

[Chain builder (initial chain governance)](./../chain-builder.md)
