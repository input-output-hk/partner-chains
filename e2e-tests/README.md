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
pytest -rP -v --blockchain substrate --env local --log-cli-level debug -vv -s -m "not active_flow and not passive_flow and not probability"
```
