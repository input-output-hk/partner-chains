# Partner Chains governance on Cardano

This document explains how to initialize and use the Governance System of Partner Chains Cardano Smart Contracts.

## Recap

Each Partner Chain is identified by a unique identifier called *genesis UTXO*.
For the same version of Smart Contracts, using different *genesis UTXO* will result in different Cardano addresses and policy ids of these contracts.

## Governance system capabilities

The Governance System sets and updates the key required to sign following transactions:
* management of *D-Parameter*
* management of *Permissioned Candidates List*
* management of *Rewards Reserve Mechanism* lifecycle: initialization, creation, and handover
* update of the Governance System itself

Governance System has to be initialized before performing any of these operations.

In the current shape of the Governance System, it allows to set or update a single key authorization.
In other words, there is one key that is required as to signing key of all of the above transactions, but it can be changed to another key.
See [Technical details](#technical-details) for more information regarding this limitation.

## Governance System Initialization

The initialization of the Governance System for the given *genesis UTXO* spends this UTXO.
This means it can be done only once for the given *genesis UTXO*.

Other than spending *genesis UTXO*, the initialization records on Cardano, that given private key is required to sign operations on smart contracts that use the Governance System.
This private key is identified by its public key hash.

### Usage

In version v1.5 command to initialize Governance System is available in the Partner Chains compatible node executable:
```
<node executable> smart-contracts governance init \
	--genesis-utxo <genesis utxo> \
	--governance-authority <governance authority key hash> \
	--payment-signing-key <path to signing key file> \
	--ogmios-url <ogmios url>
```
* `<node executable>` is the name of CLI executable that integrated Partner Chains Smart Contracts Commands
* `<genesis utxo>` is the genesis UTXO of the Partner Chain, it will be spend by the transaction, and it has to be present in the wallet belonging to the signing key
* `<governance authority key hash>` hex of blake2b-224 hash of public key related to private key that will be required to sign governance operations
* `<path to signing key file>` file should be Cardano Shelley Payment Signing Key (normal or extended)
* `<ogmios url>` is the URL of the Ogmios service connected to the Cardano node, it is optional and defaults to `ws://localhost:1337`.

Governance System initialization also takes place in the *prepare-configuration* wizard. See [Chain builder (initial chain governance)](./../chain-builder.md).

In version v1.4 this functionality is available in the smart contracts CLI application `pc-contracts-cli init-goverance`.

## Governance System update

Enables update of the key required to sign operations on smart contracts that use the Governance System.
Following its execution, the new key is required to sign operations, the old key is no longer valid for this purpose.

### Usage

In version v1.5 command to initialize governance is available in the Partner Chains compatible node executable:
```
<node executable> smart-contracts governance update \
	--genesis-utxo <genesis utxo> \
	--new-governance-authority <new governance authority key hash> \
	--payment-signing-key <path to signing key file> \
	--ogmios-url <ogmios url>
```
* `<genesis utxo>` is the genesis UTXO of the Partner Chain, it has to be the one used for `governance init`
* `<new governance authority key hash>` hex of blake2b-224 hash of public key related to private key that will be required to sign governance operations following this operation
* `<path to signing key file>` file should be Cardano Shelley Payment Signing Key (normal or extended) of the current governance authority, ie. hash of its public key should equal current governance authority hey hash
* `<ogmios url>` is the URL of the Ogmios service connected to the Cardano node, it is optional and defaults to `ws://localhost:1337`.

In version v1.4 this functionality is available in the smart contracts CLI application `pc-contracts-cli update-governance`.

## Technical details

Governance System keeps a single UTXO at the address of Governance Validator.
This UTXO has a complete script attached. Lets call it *authorization script*.
Governance system passes transactions through *authorization script*, to check if the transaction meets conditions set by this *authorization script*.

Therefore, the limitation of a single governance authority key does not stem in the smart contract of the Governance System,
but in lack of the user interface and transaction building logic that would handle multiple keys.

This means that some other UI and transaction building logic (so-called offchain code) could be implemented,
to build transactions that have properties required by the used *authorization script*.

### Links

[Rewards reserve mechanism management](./../../developer-guides/native-token-reserve-management.md)

[Chain builder (initial chain governance)](./../chain-builder.md)
