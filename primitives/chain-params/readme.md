# Partner Chains Reference Chain Parameters

`SidechainParams` is a struct that is vital for reading and verifying data of smart contracts.
Each instance of chain has its parameters, that are serialized and applied to the smart contracts on-chain code.
This means that values of the parameters influence the behavior and hashes of the smart contracts.

The structure provided in this crate matches the reference chain parameters used in `partner-chains-smart-contracts`.

**Each partner chain can have its own `SidechainParam` struct.**

Requirements for such parameters struct are:
* `ToDatum` implementation that is compatible with the smart contracts implementation of chain parameters.
`ToDatum` is compatible if the parameters are encoded in the same way.
* `clap::Parser` implementation for parsing the parameters from the command line.

## Reference implementation

The implementation provided is an example, that is compatible with the current smart contracts implementation (partner-chains-smart-contracts repository).
Compatible means that parameters structures in both implementations have the same fields and are encoded in the same way.

Parameters explained:
* `chain_id` - it serves only as unique identifier for the chain
* `governance_authority` - the hash of the public key of the governance authority,
that is allowed to change the chain parameters related to Ariadne and updating other scripts
* `threshold_numerator`, `threshold_denominator` - parameters related to the removed `pallet-active-flow`
(bidirectional bridge) crate. Because such a bridge is out of scope, these parameters are considered as legacy.
* `genesis_committee_utxo` - also related to posting certificates on Cardano.
It is utxo that is being consumed when posting the first committee hash (using smart contracts).
Related to `init` command of `partner-chains-smart-contracts`.
Since mentioned functionality is out of scope, this parameter is considered as legacy.

Legacy parameters are still present in the provided struct, for convenience of current users.
