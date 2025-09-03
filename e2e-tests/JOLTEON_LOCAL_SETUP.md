# Jolteon Local Environment Configuration

## Overview

I've successfully created a complete configuration for a local Jolteon environment that serves RPC endpoints at `127.0.0.1:9933` without requiring a Cardano node or db sync. This configuration is based on the existing Jolteon Docker environment but adapted for local development and testing.

## Files Created

### Configuration Files (`e2e-tests/config/substrate/`)

1. **`jolteon_local_nodes.json`** - Main node configuration
   - Configures 4 nodes (alice, bob, charlie, dave) all pointing to `127.0.0.1:9933`
   - Uses Jolteon-specific network parameters (--testnet-magic 2)
   - Includes all necessary node metadata (public keys, addresses, etc.)
   - Sets `test_environment` to "jolteon_local"

2. **`jolteon_local_stack.json`** - Stack configuration
   - Configures tools to run locally without Docker containers
   - Sets ogmios to localhost:1337
   - Configures substrate-node and cardano-cli paths

3. **`jolteon_local-ci.json`** - CI-specific overrides
   - Ensures all nodes point to `127.0.0.1` for CI environments
   - Minimal overrides for CI-specific configurations

### Secrets Files (`e2e-tests/secrets/substrate/jolteon_local/`)

1. **`jolteon_local.json`** - Main secrets file
   - Contains wallet configurations adapted from local environment
   - Uses Jolteon-specific addresses and keys
   - No database or Cardano-specific secrets

2. **`jolteon_local-ci.json`** - CI secrets overrides
   - Minimal overrides for CI environment

3. **`keys/`** - Directory for key files
   - Empty directory ready for any additional key files

### Documentation and Testing

1. **`jolteon_local_README.md`** - Usage documentation
   - Explains how to use the environment
   - Lists prerequisites and requirements
   - Provides example commands

2. **`test_jolteon_local_config.py`** - Configuration validation script
   - Verifies all configuration files can be loaded
   - Checks that all nodes point to localhost
   - Validates secrets directory structure

3. **`test_jolteon_local_example.py`** - Example test file
   - Demonstrates how to use the environment in tests
   - Validates configuration correctness
   - Shows proper test structure

## Key Features

### Environment Characteristics
- **RPC Endpoint**: `127.0.0.1:9933`
- **Network**: Testnet (--testnet-magic 2)
- **No Cardano Node**: Environment doesn't require Cardano node or db sync
- **No Docker**: Runs directly on localhost without containers
- **Multiple Nodes**: All nodes point to the same localhost endpoint

### Configuration Structure
- Based on Jolteon Docker environment but simplified for local use
- Maintains compatibility with existing test framework
- Supports both regular and CI environments
- Includes all necessary metadata for Jolteon-specific features

## Usage

### Running Tests
```bash
# Basic usage
pytest --env=jolteon_local --blockchain=substrate

# With CI overrides
pytest --env=jolteon_local --blockchain=substrate --ci-run

# Run specific test
pytest --env=jolteon_local --blockchain=substrate tests/test_jolteon_local_example.py
```

### Validation
```bash
# Test configuration
python e2e-tests/config/substrate/test_jolteon_local_config.py

# Run example tests
pytest --env=jolteon_local --blockchain=substrate tests/test_jolteon_local_example.py -v
```

## Prerequisites

1. **Jolteon Partner Chain Node**: Must be running locally on port 9933
2. **Network Configuration**: Node should be configured for testnet (--testnet-magic 2)
3. **Required Tools**: `substrate-node` and optionally `cardano-cli` in PATH

## Differences from Jolteon Docker Environment

| Aspect | Jolteon Docker | Jolteon Local |
|--------|----------------|---------------|
| **Host** | DNS names (alice.jolteon.sc.iog.io) | 127.0.0.1 |
| **Port** | 443 (HTTPS) | 9933 (HTTP) |
| **Protocol** | HTTPS | HTTP |
| **Docker** | Required | Not used |
| **Cardano Node** | Included | Not required |
| **DB Sync** | Included | Not required |
| **Secrets** | Complex with keys | Simplified |

## Next Steps

1. **Start Jolteon Node**: Ensure you have a Jolteon Partner Chain node running locally
2. **Test Configuration**: Run the validation script to verify setup
3. **Run Tests**: Execute tests against the local environment
4. **Customize**: Modify configuration as needed for your specific setup

The configuration is now ready to use and should work seamlessly with the existing e2e test framework!
