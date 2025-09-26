# Jolteon Local Environment

This directory contains the configuration for running Jolteon Partner Chain tests against a local environment.

## Configuration Files

- `jolteon_local_nodes.json` - Node configuration for local Jolteon environment
- `jolteon_local_stack.json` - Stack configuration for local Jolteon environment
- `jolteon_local-ci.json` - CI-specific overrides for local Jolteon environment

## Usage

To run tests against the Jolteon local environment:

```bash
# Run tests with Jolteon local environment
pytest --env=jolteon_local --blockchain=substrate

# Run tests with CI overrides
pytest --env=jolteon_local --blockchain=substrate --ci-run

# Run specific test file
pytest --env=jolteon_local --blockchain=substrate tests/test_jolteon_smoke.py
```

## Environment Details

- **RPC Endpoint**: `127.0.0.1:9933`
- **Network**: Testnet (--testnet-magic 2)
- **No Cardano Node**: This environment does not include a Cardano node or db sync
- **No Docker**: Runs directly on localhost without Docker containers

## Prerequisites

1. Ensure you have a Jolteon Partner Chain node running locally on port 9933
2. The node should be configured for testnet (--testnet-magic 2)
3. Required tools should be available in PATH:
   - `substrate-node`
   - `cardano-cli` (optional, for some tests)

## Notes

- This environment is designed for local development and testing
- All nodes point to the same localhost endpoint (127.0.0.1:9933)
- Secrets are minimal and don't include Cardano-specific keys
- The configuration is based on the Jolteon Docker environment but simplified for local use
