# Partner Chains Tests

Welcome to `Partner Chains Tests`, a powerful and flexible test automation framework for system and end-to-end (E2E) tests for partner chains.

## Pytest e2e-test Overview

### Features

- **Blockchain agnostic**
  - Execute any test against multiple blockchains! Thanks to the abstraction of `BlockchainApi`, you can write tests for different blockchains. For example, we've implemented `SubstrateApi` for Substrate-based Partner Chains, but it is possible to support other blockchains by implementing the `BlockchainApi` interface.
- **Pytest flavour**. You can write tests using well-known and one of the most popular frameworks, `pytest.`

### Partner Chains Tests - Infrastructure

![Test Infrastructure](/e2e-tests/docs/pc-tests-infra.png)

### Installation

1. Create and activate virtual environment

```bash
  pip install virtualenv
  python -m venv venv
  source venv/bin/activate
```

2. Install requirements `pip install -r requirements.txt`.
3. Install sops to [manage keys](/e2e-tests/docs/secrets.md). You can also configure [your own keys with sops](/e2e-tests/docs/configure-sops.md)

### Getting Started

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

### Examples

#### Run tests on the local environment

```bash
pytest -rP -v --blockchain substrate --env local --log-cli-level debug -vv -s -m "not probability"
```

#### Run multisig governance tests

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

### Configuration

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

---

## Continuous Integration Testing Layers

The Partner Chains CI pipeline validates each commit across four progressive testing layers, from fast local checks to long-running staging validations. Below is a breakdown of environments, coverage, and behavior across each stage.

| **Layer** | **Description** | **Validators** | **Environment** | **Test Coverage** | **Tests Run** | **Duration** | **Purpose & Details** |
|-----------|------------------|----------------|------------------|--------------------|----------------|--------------|------------------------|
| **CI Pre-Merge** | Runs on every pull request using `/dev/local-environment/`. | 5 (3 permissioned, 2 trustless) | Docker Compose | Smoke tests, RPC, metadata, committee epoch 2 | ~45 | ~7 min | Waits for epoch 2<br>Fails on skipped tests<br>Asserts node readiness and logs<br>Uses `setup.sh --non-interactive` to build full stack |
| **CI Post-Merge** | Full run post-merge on `/dev/local-environment/`. | 5 | Docker Compose | Full test suite across epochs 2, 3, 4 (except probability) | ~120 | ~10 min | Epoch-gated execution<br>Runs `--mc-epoch 3` full committee tests<br>Repeats test suite 3 times<br>Includes smart contract and native token test groups |
| **CI Preview (K8s)** | Cloud-native ephemeral test using ArgoCD-deployed `ci-preview`. | 7 (mixed) | Kubernetes | Smoke, RPC, metadata, committee sampling | ~30–40 | ~9 min | Uses `kubectl exec` to run tests inside validator pod<br>Runs `run-e2e-tests` with decrypt enabled<br>No ingress or external exposure required<br>Validates artifact compatibility with Kubernetes runtime |
| **Pre-Release (Staging)** | 2-day full soak test triggered manually via `release.yml`. | 7 | Kubernetes | All test groups: smart contracts, metadata, committee rotation, probabilities, native token | ~150+ | ~32 hrs | Deployed via ArgoCD into `staging-preview` namespace<br>Waits for finalized blocks before test execution<br>Validates across 3 mainchain epochs<br>Uses `--latest-mc-epoch` for full-cycle test coverage<br>Fails on skipped tests or misconfiguration |

---

## Continuous Integration Testing Environments

- **/dev/local-environment/**:
  - Built with `setup.sh`, which generates `.env` and `docker-compose.yml`
  - Composed of: 1 Cardano node, 1 Ogmios, 1 DB-Sync, 1 Postgres, 5 Partner Chain validator nodes
  - Setup container inserts DParam values and performs initial on-chain registration
  - Network bootstraps automatically on startup and begins block production after 2 epochs

- **ci-preview (Kubernetes)**:
  - Deployed via GitHub Actions and ArgoCD
  - Executes tests using `kubectl exec` directly into the validator container
  - Runs `run-e2e-tests` with `--decrypt` enabled
  - Artifacts are built and injected using Earthly and ECR

- **staging-preview (Kubernetes)**:
  - Used for pre-release final validation
  - 7 validator nodes run across multiple mainchain epochs
  - Blocks must be finalized before test suite execution begins
  - Includes native token tests, committee dynamics, registration, rewards, and governance
  - Results uploaded before GHCR or release publish workflows can proceed

All test layers upload full logs, metrics, and test reports to GitHub Artifacts for inspection and debugging.

---

## Continuous Integration Test Matrix

#### **Smoke Tests / Node Health**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|:---------|:---------|:--------|:----------------|:----------------------|:------------------------------|
| Block Production Advances | `test_block_producing` | Validate that node produces new blocks over time | Block height increases after 1.5x block duration sleep | Ensures block authoring is active and chain is progressing | Python SDK call to `get_latest_pc_block_number()` with timing validation |
| Basic Transaction Execution | `test_transaction` | Send transaction and verify state change | Receiver balance increases; sender balance decreases by amount + fee | Verifies signing, submission, and state application of transactions | SDK with internal signing + submit logic, validates balance changes |
| Chain Status Matches Cardano Tip | `test_get_status` | Validate that `getStatus()` aligns with Cardano CLI tip | Epoch/slot data close to Cardano tip; timestamps and sidechain data present | Confirms sync between mainchain and sidechain | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getStatus","params":[],"id":1}' http://localhost:9933` with Cardano CLI comparison |
| Genesis Params Returned | `test_get_params` | Confirm genesis config is available via RPC | `genesis_utxo` returned and correct | Ensures sidechain is initialized with correct bootstrap parameters | `curl -d '{"jsonrpc":"2.0","method":"partner_chain_getParams","params":[],"id":1}' http://localhost:9933` with genesis validation |

#### **RPC Interface Tests**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|:---------|:---------|:--------|:----------------|:----------------------|:------------------------------|
| Ariadne Parameters Structure | `test_get_ariadne_parameters` | Validate structure and presence of candidates & d-param | Correct types + keys exist for parameters with valid values | Ensures governance/consensus inputs are valid | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getAriadneParameters","params":[<epoch>],"id":1}' http://localhost:9933` with structure validation |
| Epoch Committee Present | `test_get_epoch_committee` | Verify committee members for a sidechain epoch | Valid list of members with `sidechainPubKey`s and correct count | Ensures authority resolution for epoch | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getEpochCommittee","params":[<epoch>],"id":1}' http://localhost:9933` with member validation |
| Candidate Registrations | `test_get_registrations` | Get validator registration info from RPC | List of valid, structured registrations with correct stake weights | Confirms the staking/validator registry is functioning | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getRegistrations","params":[<epoch>,"<key>"],"id":1}' http://localhost:9933` with registration validation |

#### **Committee Tests**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|:---------|:---------|:--------|:----------------|:----------------------|:------------------------------|
| Update D-Parameter | `test_update_d_param` | Update committee configuration | D-parameter updated successfully with new P and T values | Controls committee composition ratio | SDK governance call with min/max bounds validation |
| Committee Ratio Compliance | `test_epoch_committee_ratio_complies_with_dparam` | Validate committee ratio matches d-param | Ratio within calculated tolerance range based on probability simulation | Ensures fair committee composition | Statistical analysis with 50,000 simulations for tolerance calculation |
| Committee Size Compliance | `test_epoch_committee_size_complies_with_dparam` | Verify committee size matches config | Size matches min(total_committee_size, max_validators) | Ensures correct committee size | RPC + config validation with max validator bounds |
| Committee Member Rotation | `test_committee_members_rotate_over_pc_epochs` | Verify committee changes across epochs | Members rotate as expected between consecutive epochs | Prevents validator entrenchment | Epoch comparison with round-robin validation |
| Authorities Match Committee | `test_authorities_matching_committee` | Verify runtime authorities match committee | Sets match exactly with no offline validators | Ensures runtime alignment | Authority comparison with node status check |
| Update to Multisig Governance | `test_update_governance_to_multisig` | Transition to multi-key governance | Governance updated with N keys, threshold 2 | Enables secure group control | Multisig contract update with threshold validation |
| Multisig D-Parameter Update | `test_multisig_upsert_d_parameter` | Update committee config via multisig | Config updated with multiple signatures | Verifies multisig governance | Multisig transaction with witness assembly |
| Multisig Candidate Update | `test_multisig_upsert_permissioned_candidates` | Update validator set via multisig | Candidate list updated with multiple signatures | Tests multisig validator management | Multisig + witness flow with candidate validation |
| MC Epoch Committee Participation | `test_mc_epoch_committee_participation_total_number` | Verify committee participation | All slots filled with correct validator distribution | Ensures complete coverage | Participation analysis with slot validation |
| MC Epoch Participation Probability | `test_mc_epoch_committee_participation_probability` | Verify participation probabilities | Within expected bounds based on stake weight | Ensures fair selection | Probability analysis with stake-weighted validation |
| Active Trustless Participation | `test_active_trustless_candidates_were_in_committee` | Verify active trustless participation | Found in committee with correct stake delegation | Confirms active status | Committee membership check with stake validation |
| Inactive Trustless Exclusion | `test_inactive_trustless_candidates_were_not_in_committee` | Verify inactive trustless exclusion | Not in committee after deactivation epoch | Validates deactivation | Committee membership check with epoch validation |
| Active Permissioned Participation | `test_active_permissioned_candidates_were_in_committee` | Verify active permissioned participation | Found in committee with correct weight | Confirms active status | Committee membership check with weight validation |
| Inactive Permissioned Exclusion | `test_inactive_permissioned_candidates_were_not_in_committee` | Verify inactive permissioned exclusion | Not in committee after deactivation epoch | Validates deactivation | Committee membership check with epoch validation |
| At Least One Trustless | `test_there_is_at_least_one_trustless_candidate` | Verify trustless presence | At least one found with valid stake | Ensures decentralization | Candidate enumeration with stake validation |
| At Least One Permissioned | `test_there_is_at_least_one_permissioned_candidate` | Verify permissioned presence | At least one found with valid weight | Ensures trusted validators | Candidate enumeration with weight validation |
| No Rogue Committee Members | `test_no_rogue_committee_members` | Verify all members are valid | No unexpected members in active set | Ensures committee integrity | Member validation against active candidates |
| Block Authors Match Committee | `test_block_authors_match_committee_seats` | Validate authorship against committee list | All blocks authored by expected keys in round-robin order | Prevents unauthorized block production | Slot-based validator key matching with order validation |
| Block Header Has MC Hash | `test_block_headers_have_mc_hash` | Confirm mainchain hash is embedded in each block | Hash is not null, and stable block is valid | Ensures fork choice and sync safety | Header inspection logic with hash validation |
| Validator Metadata Upsert | `test_block_producer_can_update_their_metadata` | Submit validator metadata and confirm storage | Metadata returned from storage + RPC match submission | Ensures public validator data is available and updatable | SDK + RPC query with signature validation |
| Register Trustless Candidate | `test_register_candidate` | Onboard new trustless validator | Appears in `getAriadneParameters` after activation epoch | Tests initial state entry for validator set | SDK-driven registration with epoch validation |
| Deregister Trustless Candidate | `test_deregister_candidate` | Remove a validator from the active set | Candidate disappears from registration RPC | Ensures clean offboarding path | SDK command + epoch delay verification |
| Upsert Permissioned Candidates | `test_upsert_permissioned_candidates` | Change set of trusted validators | List matches update post-epoch | Allows governance reconfiguration | Combined SDK+DB assertion with epoch validation |
| Governance Policy Exists | `test_get_governance_policy` | Validate initial governance state | 1 key, threshold = 1 | Baseline authority model for smart contract ops | `governance.get_policy()` via SDK with threshold check |
| Verify Multisig Governance | `test_verify_multisig_policy` | Confirm multisig config was correctly applied | N keys, threshold 2 | Guarantees governance config is correct | Policy verification with key count validation |

#### **Delegator Rewards Tests**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|:---------|:---------|:--------|:----------------|:----------------------|:------------------------------|
| Block Participation Data | `test_block_participation_data_is_not_empty` | Verify participation data exists | Data present and valid with up_to_slot and producer_participation | Basis for rewards calculation | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getBlockParticipationData","params":[block_hash],"id":1}' http://localhost:9933` with data structure validation |
| Pro Bono Participation | `test_pro_bono_participation` | Verify permissioned validator participation | Present in logs with correct block count and zero delegator shares | Ensures trusted validator contribution | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getPermissionedCandidates","params":[mc_epoch],"id":1}' http://localhost:9933` with block count validation |
| SPO Participation | `test_spo_participation` | Verify SPO participation | Blocks produced by SPO with correct delegator shares | Confirms SPO contribution | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getTrustlessCandidates","params":[mc_epoch],"id":1}' http://localhost:9933` with stake delegation validation |
| No Unexpected Producers | `test_no_unexpected_producers` | Verify only valid producers | No unauthorized producers in participation data | Ensures chain security | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getBlockParticipationData","params":[block_hash],"id":1}' http://localhost:9933` with producer validation |
| Delegator Journey | `test_delegator_journey` | Complete delegator registration | Successfully registered with valid stake key | Enables delegation | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getAddressAssociation","params":[stake_key_hash],"id":1}' http://localhost:9933` with registration validation |
| Delegator Address Association | `test_delegator_can_associate_pc_address` | Bind stake address to sidechain | Association confirmed with valid signature | Enables rewards routing | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getAddressAssociation","params":[stake_key_hash],"id":1}' http://localhost:9933` with signature validation |
| Block Production Log | `test_block_production_log` | Verify block production logging | Logs present and valid with correct author mapping | Tracks block authorship | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getBlockProductionLog","params":[block_hash],"id":1}' http://localhost:9933` with author validation |
| Delegator Rewards Distribution | `test_delegator_rewards` | Verify rewards are distributed to delegators | Rewards match expected values based on stake weight | Ensures economic incentives work | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getDelegatorRewards","params":[stake_key_hash],"id":1}' http://localhost:9933` with stake-weighted validation |
| SPO Rewards | `test_spo_rewards` | Verify SPO receives rewards | Rewards match expected values based on block production | Ensures economic incentives work | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getSPORewards","params":[spo_key],"id":1}' http://localhost:9933` with block count validation |
| Block Production Log Pallet | `test_block_production_log_pallet` | Verify block production log is populated | Log entries match expected authors with correct SPO mapping | Ensures accurate block authorship tracking | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getBlockProductionLog","params":[block_hash],"id":1}' http://localhost:9933` with SPO validation |
| Block Production Log Structure | `test_block_production_log_structure` | Verify log structure is correct | Log entries have expected fields with valid author types | Ensures log data is usable | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getBlockProductionLog","params":[block_hash],"id":1}' http://localhost:9933` with structure validation |

#### **Smart Contract Tests**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|:---------|:---------|:--------|:----------------|:----------------------|:------------------------------|
| Init Reserve | `test_init_reserve` | Deploy reserve contracts | Contracts deployed with validator and policy scripts | Bootstraps economic layer | `curl -d '{"jsonrpc":"2.0","method":"sidechain_initReserve","params":[payment_key],"id":1}' http://localhost:9933` with script validation |
| Create Reserve | `test_create_reserve` | Initialize reserve with funds | Reserve funded with correct initial deposit | Starts token issuance | `curl -d '{"jsonrpc":"2.0","method":"sidechain_createReserve","params":[v_function_hash,initial_deposit,token,payment_key],"id":1}' http://localhost:9933` with balance validation |
| Release Funds | `test_release_funds` | Move tokens to circulation | Tokens released with correct reference UTXO | Enables token spending | `curl -d '{"jsonrpc":"2.0","method":"sidechain_releaseFunds","params":[reference_utxo,amount,payment_key],"id":1}' http://localhost:9933` with UTXO validation |
| Deposit Funds | `test_deposit_funds` | Return tokens to reserve | Tokens deposited with correct amount | Supports token locking | `curl -d '{"jsonrpc":"2.0","method":"sidechain_depositFunds","params":[amount,payment_key],"id":1}' http://localhost:9933` with balance validation |
| Handover Reserve | `test_handover_reserve` | Transfer entire reserve | Reserve transferred with zero balance | Handles lifecycle events | `curl -d '{"jsonrpc":"2.0","method":"sidechain_handoverReserve","params":[payment_key],"id":1}' http://localhost:9933` with balance validation |
| Native Token Init | `test_native_token_init` | Initialize native token | Token initialized with correct policy | Enables token operations | `curl -d '{"jsonrpc":"2.0","method":"sidechain_initNativeToken","params":[payment_key],"id":1}' http://localhost:9933` with policy validation |
| Native Token Transfer | `test_native_token_transfer` | Transfer native token | Transfer successful with correct balances | Verifies token functionality | `curl -d '{"jsonrpc":"2.0","method":"sidechain_transferNativeToken","params":[from,to,amount,payment_key],"id":1}' http://localhost:9933` with balance validation |
| Native Token Balance | `test_native_token_balance` | Check token balance | Balance correct with proper accounting | Ensures token accounting | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getNativeTokenBalance","params":[address],"id":1}' http://localhost:9933` with accounting validation |

#### **Error Handling Tests**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|:---------|:---------|:--------|:----------------|:----------------------|:------------------------------|
| Invalid Transaction | `test_invalid_transaction` | Submit invalid transaction | Transaction rejected with proper error code | Ensures invalid transactions are rejected | `curl -d '{"jsonrpc":"2.0","method":"sidechain_submitTransaction","params":[invalid_tx],"id":1}' http://localhost:9933` with error validation |
| Network Failure Recovery | `test_network_failure_recovery` | Simulate network failure | Node recovers and continues with correct state | Ensures network resilience | Network simulation with `docker-compose stop node1 && docker-compose start node1` and state validation |
| Invalid Input Handling | `test_invalid_input_handling` | Submit invalid input | Error returned with proper message | Ensures proper error handling | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getStatus","params":[invalid_param],"id":1}' http://localhost:9933` with error validation |

#### **Performance Tests**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|:---------|:---------|:--------|:----------------|:----------------------|:------------------------------|
| Transaction Throughput | `test_transaction_throughput` | Measure transaction throughput | Throughput meets requirements with batch processing | Ensures system can handle load | `curl -d '{"jsonrpc":"2.0","method":"sidechain_submitTransaction","params":[tx],"id":1}' http://localhost:9933` with batch timing |
| Block Production Speed | `test_block_production_speed` | Measure block production speed | Speed meets requirements with consistent timing | Ensures timely block production | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getBlock","params":[block_number],"id":1}' http://localhost:9933` with timing validation |
| Network Latency | `test_network_latency` | Measure network latency | Latency within acceptable range with consistent response | Ensures responsive network | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getStatus","params":[],"id":1}' http://localhost:9933` with timing validation |

#### **Security Tests**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|:---------|:---------|:--------|:----------------|:----------------------|:------------------------------|
| Unauthorized Access | `test_unauthorized_access` | Attempt unauthorized access | Access denied with proper error | Ensures proper access control | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getStatus","params":[],"id":1}' http://localhost:9933` with invalid credentials |
| Replay Attack | `test_replay_attack` | Attempt replay attack | Attack fails with proper error | Ensures transaction uniqueness | `curl -d '{"jsonrpc":"2.0","method":"sidechain_submitTransaction","params":[old_tx],"id":1}' http://localhost:9933` with nonce validation |
| Double Spend | `test_double_spend` | Attempt double spend | Transaction rejected with proper error | Ensures transaction uniqueness | `curl -d '{"jsonrpc":"2.0","method":"sidechain_submitTransaction","params":[same_tx],"id":1}' http://localhost:9933` with UTXO validation |

#### **Upgrade Tests**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|:---------|:---------|:--------|:----------------|:----------------------|:------------------------------|
| Chain Upgrade | `test_chain_upgrade` | Upgrade chain | Upgrade successful with version check | Ensures smooth upgrades | `curl -d '{"jsonrpc":"2.0","method":"sidechain_upgradeChain","params":[version,payment_key],"id":1}' http://localhost:9933` with version validation |
| Smart Contract Upgrade | `test_smart_contract_upgrade` | Upgrade smart contract | Upgrade successful with code validation | Ensures contract upgrades work | `curl -d '{"jsonrpc":"2.0","method":"sidechain_upgradeContract","params":[contract_address,new_code,payment_key],"id":1}' http://localhost:9933` with code validation |
| Backward Compatibility | `test_backward_compatibility` | Verify backward compatibility | Compatibility maintained with version check | Ensures upgrades don't break existing functionality | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getStatus","params":[],"id":1}' http://localhost:9933` with version check |

#### **Network Partition Tests**

| Test Name | Function | Purpose | Expected Result | Why This Test Matters | How Test is Run / RPC Example |
|:---------|:---------|:--------|:----------------|:----------------------|:------------------------------|
| Network Partition | `test_network_partition` | Simulate network partition | Partition handled correctly with consensus | Ensures network resilience | Network simulation with `docker-compose stop node1 node2 && docker-compose start node1 node2` and consensus validation |
| Node Recovery | `test_node_recovery` | Recover node after failure | Node recovers and syncs with correct state | Ensures node resilience | `docker-compose stop node1 && docker-compose start node1` with state validation |
| Consensus Recovery | `test_consensus_recovery` | Recover consensus after partition | Consensus restored with correct chain state | Ensures consensus resilience | `curl -d '{"jsonrpc":"2.0","method":"sidechain_getStatus","params":[],"id":1}' http://localhost:9933` after partition with state validation |

For more details on how to implement Native Token Reserve Management in a partner chain, refer to the [Native Token Migration Guide](docs/developer-guides/native-token-migration-guide.md)

### Test Execution

Tests are executed in CI using the `run-e2e-tests` action, which runs the following test categories:

```yaml
test_categories:
  - smoke
  - committee
  - delegator_rewards
  - smart_contracts
  - rpc
```

Each category is run with appropriate test markers and configurations to ensure comprehensive coverage of the Partner Chain functionality.

