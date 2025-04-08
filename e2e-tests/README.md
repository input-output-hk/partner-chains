# Partner Chains Tests

Welcome to `Partner Chains Tests`, a powerful and flexible test automation framework for system and end-to-end (E2E) tests for partner chains.

## Features

- **Blockchain agnostic**
  - Execute any test against multiple blockchains! Thanks to the abstraction of `BlockchainApi`, you can write tests for different blockchains. For example, we've implemented `SubstrateApi` for Substrate-based Partner Chains, but it is possible to support other blockchains by implementing the `BlockchainApi` interface.
- **Pytest flavour**. You can write tests using well-known and one of the most popular frameworks, `pytest.`

## Partner Chains Tests - Infrastructure

![Test Infrastructure](/e2e-tests/docs/pc-tests-infra.png)

## Installation

1. Create and activate virtual environment

```bash
  pip install virtualenv
  python -m venv venv
  source venv/bin/activate
```

2. Install requirements `pip install -r requirements.txt`.
3. Install sops to [manage keys](/e2e-tests/docs/secrets.md). You can also configure [your own keys with sops](/e2e-tests/docs/configure-sops.md)

## Getting Started

- Choose an environment to run tests. You have an option to run on [local](/e2e-tests/docs/run-tests-on-local-env.md) or [your own custom](/e2e-tests/docs/run-tests-on-new-env.md) environments
- Run `pytest -h` to see all available options, or simply `pytest` to execute all tests.

### Execution Options

```bash
Custom options:
  --ctrf=CTRF           generate test report. Report file name is optional
  --env=ENV             Target node environment
  --blockchain={substrate,midnight}
                        Blockchain network type
  --ci-run              Overrides config values specific for executing from ci runner
  --decrypt             Decrypts secrets and keys files
  --node-host=NODE_HOST
                        Overrides node host
  --node-port=NODE_PORT
                        Overrides node port
  --init-timestamp=INIT_TIMESTAMP
                        Initial timestamp of the mainchain.
  --latest-mc-epoch     Parametrize committee tests to verify whole last MC epoch. Transforms sc_epoch param to range of SC epochs for last MC epoch.
  --mc-epoch=MC_EPOCH   MC epoch that parametrizes committee tests to verify the whole given MC epoch. Translates sc_epoch param to range of SC epochs for given MC epoch.
  --sc-epoch=SC_EPOCH   SC epoch that parametrizes committee tests, default: <last_sc_epoch>.
```

## Examples

### Run tests on the local environment

```bash
pytest -rP -v --blockchain substrate --env local --log-cli-level debug -vv -s -m "not probability"
```

### Run multisig governance tests

To test the multisig governance functionality, you need to configure additional governance authorities in your configuration file. The tests will verify both single signature and multisig workflows.

```bash
pytest -rP -v --blockchain substrate --env local -m "multisig_governance"
```

The multisig tests verify:
1. Updating governance to use multiple authorities with a threshold of required signatures
2. Testing multisig operations for various governance actions (D parameter, permissioned candidates, reserve operations)
3. Creating, signing, and submitting transactions with multiple signatures
4. Restoring governance back to the original single key setup after tests complete

This test workflow ensures that the environment is left in the same state it started with, so that other tests that expect single-key governance will continue to work correctly.

#### Configuration

For multisig testing, add the following to your configuration file:

```yaml
nodes_config:
  governance_authority:
    mainchain_address: "main_authority_address"
    mainchain_key: "path/to/main_authority.skey"
  additional_governance_authorities:
    - "path/to/second_authority.skey"
    - "path/to/third_authority.skey"
```

The `additional_governance_authorities` should be a list of paths to the signing key files for additional authorities.


### End-to-End Test Matrix

---

#### **Smoke Tests / Node Health**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|-----------|----------|---------|------------------|------------------------|------------------------------|
| Block Production Advances | `test_block_producing` | Validate that node produces new blocks over time | Block height increases after 1.5x block duration sleep | Ensures block authoring is active and chain is progressing | Python SDK call to `get_latest_pc_block_number()` |
| Basic Transaction Execution | `test_transaction` | Send transaction and verify state change | Receiver balance increases; sender balance decreases by amount + fee | Verifies signing, submission, and state application of transactions | SDK with internal signing + submit logic |
| Chain Status Matches Cardano Tip | `test_get_status` | Validate that `getStatus()` aligns with Cardano CLI tip | Epoch/slot data close to Cardano tip; timestamps and sidechain data present | Confirms sync between mainchain and sidechain | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getStatus","params":[],"id":1}' http://localhost:9933` |
| Genesis Params Returned | `test_get_params` | Confirm genesis config is available via RPC | `genesis_utxo` returned and correct | Ensures sidechain is initialized with correct bootstrap parameters | `curl -d '{"jsonrpc":"2.0","method":"partner_chain_getParams","params":[],"id":1}' http://localhost:9933` |

---

#### **RPC Interface Tests**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|-----------|----------|---------|------------------|------------------------|------------------------------|
| Ariadne Parameters Structure | `test_get_ariadne_parameters` | Validate structure and presence of candidates & d-param | Correct types + keys exist for parameters | Ensures governance/consensus inputs are valid | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getAriadneParameters","params":[<epoch>],"id":1}' http://localhost:9933` |
| Epoch Committee Present | `test_get_epoch_committee` | Verify committee members for a sidechain epoch | Valid list of members with `sidechainPubKey`s | Ensures authority resolution for epoch | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getEpochCommittee","params":[<epoch>],"id":1}' http://localhost:9933` |
| Candidate Registrations | `test_get_registrations` | Get validator registration info from RPC | List of valid, structured registrations | Confirms the staking/validator registry is functioning | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getRegistrations","params":[<epoch>,"<key>"],"id":1}' http://localhost:9933` |

---

#### **Registration & Metadata Tests**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|-----------|----------|---------|------------------|------------------------|------------------------------|
| Validator Metadata Upsert | `test_block_producer_can_update_their_metadata` | Submit validator metadata and confirm storage | Metadata returned from storage + RPC match submission | Ensures public validator data is available and updatable | SDK + RPC query to `partner_chain_getBlockProducerMetadata(pubkey)` |
| Register Trustless Candidate | `test_register_candidate` | Onboard new trustless validator | Appears in `getAriadneParameters` after activation epoch | Tests initial state entry for validator set | SDK-driven, verified via RPC |
| Deregister Trustless Candidate | `test_deregister_candidate` | Remove a validator from the active set | Candidate disappears from registration RPC | Ensures clean offboarding path | SDK command + epoch delay verification |
| Upsert Permissioned Candidates | `test_upsert_permissioned_candidates` | Change set of trusted validators | List matches update post-epoch | Allows governance reconfiguration | Combined SDK+DB assertion |
| Delegator Can Associate Address | `test_delegator_can_associate_pc_address` | Bind Cardano stake address to sidechain account | Confirmed via query | Enables delegation and rewards routing | RPC: `submit_address_association`, `get_address_association` |

---

#### **Governance & Multisig**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|-----------|----------|---------|------------------|------------------------|------------------------------|
| Governance Policy Exists | `test_get_governance_policy` | Validate initial governance state | 1 key, threshold = 1 | Baseline authority model for smart contract ops | `governance.get_policy()` via SDK |
| Switch to Multisig Governance | `test_update_governance_to_multisig` | Transition to multi-key control | Successful tx submitted | Enables secure group control | Multisig smart contract update via SDK |
| Verify Multisig Governance | `test_verify_multisig_policy` | Confirm multisig config was correctly applied | N keys, threshold 2 | Guarantees governance config is correct | Same as above |
| Upsert DParam via Multisig | `test_multisig_upsert_d_parameter` | Update committee config under multisig | On-chain DParam matches update | Verifies governance control over consensus | Witness-signed tx |
| Upsert Candidates via Multisig | `test_multisig_upsert_permissioned_candidates` | Change validator set via multisig | Permissioned list updated correctly | Allows policy-governed membership | Multisig + witness flow |

---

#### **Consensus / Committee Rotation**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|-----------|----------|---------|------------------|------------------------|------------------------------|
| Block Authors Match Committee | `test_block_authors_match_committee_seats` | Validate authorship against committee list | All blocks authored by expected keys | Prevents unauthorized block production | Slot-based validator key matching |
| Block Header Has MC Hash | `test_block_headers_have_mc_hash` | Confirm mainchain hash is embedded in each block | Hash is not null, and stable block is valid | Ensures fork choice and sync safety | Header inspection logic |
| Committee Members Rotate | `test_committee_members_rotate_over_pc_epochs` | Confirm committee changes across PC epochs | Pubkey set changes over epochs | Protects against validator entrenchment | Epoch-to-epoch comparison |
| Authorities Match Committee | `test_authorities_matching_committee` | Ensure runtime authority list matches committee | Equal validator sets | Ensures runtime is aligned with governance-set committee | SDK-composed logic |

---

#### **DParam & Committee Ratio Validations**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|-----------|----------|---------|------------------|------------------------|------------------------------|
| Committee Participation Ratio Matches DParam | `test_epoch_committee_ratio_complies_with_dparam` | Ratio of permissioned/trustless matches d-param | Ratio in tolerance range | Protects fairness guarantees | Statistical simulation logic |
| Committee Size Matches DParam | `test_epoch_committee_size_complies_with_dparam` | Committee size == permissioned + trustless | Size matches expectation | Ensures config alignment | RPC + DParam pull |
| MC Epoch Attendance Consistent | `test_mc_epoch_committee_participation_total_number` | Validate that every slot was filled | Committee seats * epochs = attendance | Detects validator gaps | DB vs. expected math |
| MC Epoch Probabilities Normalized | `test_mc_epoch_committee_participation_probability` | Probabilities and attendance within bounds | Tolerance-checked | Confirms fairness of selection | DB+math consistency |

---

#### **Candidate Activity & Rotation**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|-----------|----------|---------|------------------|------------------------|------------------------------|
| Active Trustless Candidates Participated | `test_active_trustless_candidates_were_in_committee` | Ensure candidates marked active participated | Count > 0 | Ensures active nodes contribute | DB lookup |
| Inactive Trustless Candidates Didn't Participate | `test_inactive_trustless_candidates_were_not_in_committee` | Deregistered trustless not in committee | Count = 0 | Validates pruning logic | Epoch scan |
| Active Permissioned Candidates Participated | `test_active_permissioned_candidates_were_in_committee` | Same as above for permissioned | Found in committee | Confirms inclusion | DB + committee |
| Inactive Permissioned Candidates Didn't Participate | `test_inactive_permissioned_candidates_were_not_in_committee` | Removed permissioned don’t appear | Absent from committee | Confirms removal | Same as above |

---

#### **Block Participation & Rewards Basis**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|-----------|----------|---------|------------------|------------------------|------------------------------|
| Block Participation Data Exists | `test_block_participation_data_is_not_empty` | Confirm raw participation data is populated | Slots + producers available | Basis for rewards calculation | Test helper RPC |
| Pro Bono Participation Valid | `test_pro_bono_participation` | Ensure permissioned validators participated | Present in logs | Guarantees contribution from trusted | Validator to log match |
| SPO Participation Valid | `test_spo_participation` | Same as above, for trustless | Stake key + produced blocks recorded | Enables rewards sharing | Stake table joins with logs |
| No Unexpected Producers | `test_no_unexpected_producers` | Catch rogue/unregistered authors | No extra entries in logs | Chain safety check | Diff producer set vs. known validators |

---

#### **Smart Contract Tests: Reserve & Circulation**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|-----------|----------|---------|------------------|------------------------|------------------------------|
| Init Reserve Contracts | `test_init_reserve` | Deploy Reserve, Policy, Circulation scripts | Tx ID returned | Bootstraps economic layer | CLI deployment |
| Create Reserve | `test_create_reserve` | Transfer initial funds to reserve script | Wallet debited, reserve funded | Start point for token issuance | Token balance logic |
| Release Funds from Reserve | `test_release_funds` | Move tokens to circulation validator | Reserve down, circulation up | Enables spendability of locked supply | CLI smart contract call |
| Deposit Funds to Reserve | `test_deposit_funds` | Return tokens to Reserve | Balances update accordingly | Supports token re-locking | Token accounting |
| Handover Reserve | `test_handover_reserve` | Flush entire reserve to circulation | Reserve = 0; circulation = old reserve | Required in certain lifecycle events | CLI batch call |


