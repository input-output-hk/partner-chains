# Partner Chains Governance on Cardano

This document explains how to initialize and use the Governance System of Partner Chains Cardano Smart Contracts.

## Recap

Each Partner Chain is identified by a unique identifier called *genesis UTXO*.
For the same version of Smart Contracts, using a different *genesis UTXO* will result in different Cardano addresses and policy ids of these contracts.

## Governance System Capabilities

The Governance System sets and updates the key(s) required to sign the following transactions:

* management of *D-Parameter*
* management of *Permissioned Candidates List*
* management of *Rewards Reserve Mechanism* lifecycle: initialization, creation, and handover
* update of the Governance System itself
* management of *Governed Map*

The Governance System has to be initialized before performing any of these operations.

## Governance System Initialization

The initialization of the Governance System for a given *genesis UTXO* spends that UTXO.
This means it can be done only once for a given *genesis UTXO*.

Other than spending *genesis UTXO*, the initialization records on Cardano, that given private keys are required to sign operations on smart contracts that use the Governance System.
These private keys are identified by their public key hashes.

### Usage

In version v1.7 the command to initialize Governance System is available in the Partner Chains compatible node executable:

```bash
./partner-chains-node smart-contracts governance init \
	--genesis-utxo <GENESIS_UTXO> \
	--ogmios-url <OGMIOS_URL> \
	--payment-key-file <PAYMENT_KEY_FILE> \
	--governance-authority <GOVERNANCE_AUTHORITY> \
	--threshold <THRESHOLD>
```

* `<GENESIS_UTXO>`: The genesis UTXO of the Partner Chain, it will be spend by the transaction, and it has to be present in the wallet belonging to the signing key.
* `<OGMIOS_URL>`: The URL of the Ogmios service connected to the Cardano node, it is optional and defaults to `ws://localhost:1337`.
* `<PAYMENT_KEY_FILE>`: Cardano Shelley Payment Signing Key file (normal or extended).
* `<GOVERNANCE_AUTHORITY>`: Hex encoded blake2b-224 public keys hashes related to the private keys that will be required to sign governance operations.
* `<THRESHOLD>`: the number of distinct signatures that will be required to sign governance operations

Governance System initialization also takes place in the *prepare-configuration* wizard. See [Chain builder (initial chain governance)](./../chain-builder.md).

In version v1.5 and v1.6 only single governance key is supported.

In version v1.4 this functionality is available in the smart contracts CLI application `pc-contracts-cli init-goverance`.

## Governance System Update

Enables update of the keys required to sign operations on smart contracts that use the Governance System.
Following its execution, the new key is required to sign operations, the old keys are no longer valid for this purpose.

### Usage

In version v1.5 command to initialize governance is available in the Partner Chains compatible node executable:
```
./partner-chains-node smart-contracts governance update \
	--genesis-utxo <GENESIS_UTXO> \
	--ogmios-url <OGMIOS_URL> \
	--payment-key-file <PAYMENT_KEY_FILE> \
	--governance-authority <NEW_GOVERNANCE_AUTHORITY> \
	--threshold <NEW_GOVERNANCE_THRESHOLD>
```

* `<GENESIS_UTXO>`: The genesis UTXO of the Partner Chain. Same as the one used for `governance init`.
* `<OGMIOS_URL>`: The URL of the Ogmios service connected to the Cardano node, it is optional and defaults to `ws://localhost:1337`.
* `<PAYMENT_KEY_FILE>`: Cardano Shelley Payment Signing Key file (normal or extended) of the current governance authority (ie. hash of its public key should equal current governance authority hey hash).
* `<NEW_GOVERNANCE_AUTHORITY>`: List of hex encoded blake2b-224 hashes of public keys related to private keys that will be required to sign governance operations following this operation. Multiple keys can be provided, separated by spaces.
* `<NEW_GOVERNANCE_THRESHOLD>`: Number of keys required to sign a transaction.

In version v1.4 this functionality is available in the smart contracts CLI application `pc-contracts-cli update-governance`.

## Multi Signature Governance

All the `smart-contracts` sub-commands that require Governance: `governance update`, `upsert-d-parameter`, `upsert-permissioned-candidates`, `reserve init|create|deposit|handover|update-settings`, and `governed-map insert|update|remove` will now submit the transaction only if the governance is "1 of 1". Otherwise these commands return a transaction CBOR that can be submitted with the new command `assemble-and-submit-tx`. Signatures can be obtained using `sign-tx`. Example of executed commands, invoked by owners of `key1` and `key2` are:

Owner of `key1` initialized Governance with two key hashes `e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b` and `7fa48bb8fb5d6804fad26237738ce490d849e4567161e38ab8415ff3`(that are hashes of `key1` and `key2`), and sets requried number of signatures to `2`.
```
./partner-chains-node smart-contracts governance init -c f8fbe7316561e57de9ecd1c86ee8f8b512a314ba86499ba9a584bfa8fe2edc8d#0 \
-g e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b 7fa48bb8fb5d6804fad26237738ce490d849e4567161e38ab8415ff3 \
-t 2 \
-k key1.skey
...
{
  "tx_hash": "0x4a9567757eb9000ac5cd3e69e09551893084170ba2598bd26d3d25e1bcd0fb6c",
  "genesis_utxo": "f8fbe7316561e57de9ecd1c86ee8f8b512a314ba86499ba9a584bfa8fe2edc8d#0"
}
```

Owner of `key1` wants to set D-parameter to (7, 5).
```
./partner-chains-node smart-contracts upsert-d-parameter \
-c f8fbe7316561e57de9ecd1c86ee8f8b512a314ba86499ba9a584bfa8fe2edc8d#0 \
--permissioned-candidates-count 7 \
--registered-candidates-count 5 \
-k key1.skey | jq .
...
{"transaction_to_sign":{"temporary_wallet":{"address":"addr_test1vzeg2g6gcnlvnemk9hgvsaxxktf8suxwd63hm54w9erxuwc49exyq","funded_by_tx":"0xed99c5eb6d12053c514915fcb0445c9ce9839b65570db042fcd1c9d9cc9fbcf8","private_key":"0x730f9c6f26666da41dedbe596f6b2f7d36a98ce768591010b537e4f48417448f"},"tx":{"cborHex":"84aa00..transaction bytes redacted..f6","description":"","type":"Tx ConwayEra"},"tx_name":"Insert D-parameter"}}
```
The user gets the transaction data. It already contains the `key1` signature. Transaction requires signature of `key2` owner, before it can be submitted.

`key2` owner has to get this transaction data, perhaps from `key1` owner, *inspect the transaction* and then sign `cborHex` value from the previous output:
```
./partner-chains-node smart-contracts sign-tx -k key2.skey --transaction 84aa00..<transaction-bytes-redacted>..f6
...
{"cborHex":"82008258202bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c58409dfff5d837ec7b864502c7acac5ad5885f74d94cb68458413ee4565ff52f6dcb1ff3df272566662b4f00766fc9586a12532bfce68e56280f93dd57d6e22b9705","description":"","type":"TxWitness ConwayEra"}
```
This time, `cborHex` contains the missing signature that is required for the next step, which is the transaction submission `assemble-and-submit-tx`.
The user has to provide signatures in `-w` parameter:
```
./partner-chains-node smart-contracts assemble-and-submit-tx --transaction 84aa00..transaction bytes redacted..f6 -w 82008258202bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c58409dfff5d837ec7b864502c7acac5ad5885f74d94cb68458413ee4565ff52f6dcb1ff3df272566662b4f00766fc9586a12532bfce68e56280f93dd57d6e22b9705
```

`assemble-and-submit-tx` and `sign-tx` are added for unified UX. Signing and transaction submission can be done in other ways as well.

`governance get-policy` subcommand prints the current Governance Policy.

### Details of creating transaction to sign

Procedure of creating transaction to sign is as follows:
* a temporary wallet is generated
* temporary wallet private key is saved to a file
* `--payment-key` transfers required funds to the temporary wallet
* a transaction paid from this temporary wallet is created
* transaction and temporary wallet data are printed to stdout.

## Technical details
The Governance System was designed to be quite flexible. To achieve this it keeps a single UTXO at the address of Governance Validator.
This UTXO has a complete script attached (*authorization script*).
The Governance system passes transactions through the *authorization script*, to check if the transaction meets conditions set by this *authorization script*.

CLI implemented by Partner Chains uses "M of N" MultiSig script. Such a script requires that transaction has valid signatures of at least M out of N keys. M parameter (threshold) and a list of N public keys are applied, in Plutus meaning, to the base script and then stored at the Governance System validator address.

CLIs distinguish two situations:
* when MultiSig requires only a single signature and the payment key is one of the governance keys, then it submits transactions to Cardano
* when MultiSig requires more signatures, then it instead print out transaction CBOR that has to be signed by more keys, and submitted later - see [Multi Signature Governance]

## Links

[Rewards reserve mechanism management](./../../developer-guides/native-token-reserve-management.md)

[Chain builder (initial chain governance)](./../chain-builder.md)

[Governed Map](../../../toolkit/governed-map/README.md)
