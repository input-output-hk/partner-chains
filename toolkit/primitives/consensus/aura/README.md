# Partner Chains Aura Consensus primitives

This crate provides primitives used by Partner Chains Aura Consensus.
Items provided:
* `CurrentSlotProvider` for consensus to know current slot according to wall-clock
* `InherentDigest` for consensus to digest InherentData and compare this digest block import
* `PartnerChainsProposer` is a Proposer that additionally adds inherent data digests to logs

License: Apache-2.0
