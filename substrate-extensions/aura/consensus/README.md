# Partner Chains Aura Workers

This crate is base on the [Substrate Aura Consensus crate](https://github.com/paritytech/polkadot-sdk/tree/polkadot-stable2407/substrate/client/consensus/aura).
It is customized for Partner Chains needs:
* during block verification it uses block slot to call Partner Chains InherentDataProvider
* verifies that the given block header has proper InherentDigest (digest of data from Partner Chains InherentDataProvider).

Please note that it requires usage of custom `Proposer` that comes in `sp-partner-chains-consensus-aura` crate.
See `service.rs` in the `node` crate to see how to use it.

License: GPL-3.0-or-later WITH Classpath-exception-2.0
