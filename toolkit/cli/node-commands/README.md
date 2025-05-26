# Partner Chains Node Commands

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

The `partner-chains-node-commands` crate provides a comprehensive set of command-line interface (CLI) commands for interacting with Partner Chains nodes. This crate serves as the primary interface for node operators, validators, and developers to manage and interact with Partner Chain infrastructure.

## Overview

Partner Chains are application-specific blockchains that leverage Cardano's security and decentralization while providing enhanced functionality and performance for specific use cases. This crate provides the essential CLI tooling to:

- Query sidechain parameters and status
- Manage validator registration and committee membership
- Handle cryptographic signatures for various operations
- Interact with Partner Chain smart contracts on Cardano
- Access setup wizards for chain configuration

## Features

### Core Commands

#### 1. Sidechain Parameters (`sidechain-params`)
Retrieves fundamental sidechain parameters including genesis UTXO and configuration details.

**Usage:**
```bash
partner-chains-node sidechain-params
```

#### 2. Registration Status (`registration-status`)
Checks the registration status of a validator for a given mainchain public key and epoch number. This is crucial for validator operators to verify their registration has been successfully processed on Cardano.

**Usage:**
```bash
partner-chains-node registration-status \
  --stake-pool-pub-key 0x702b81ab2e86cf73a87062af1eb0da666d451976d9d91c63a119ed94e6a33dc0 \
  --mc-epoch-number 586
```

**Important Notes:**
- If registration was included in Cardano block in epoch N, it should be visible when querying epoch N+1 or later
- If registration doesn't appear after a few minutes, debugging may be required

#### 3. Ariadne Parameters (`ariadne-parameters`)
Retrieves Ariadne protocol parameters that are effective at a specific mainchain epoch. These parameters control various aspects of the Partner Chain consensus mechanism.

**Usage:**
```bash
partner-chains-node ariadne-parameters --mc-epoch-number 586
```

**Note:** Parameters become effective two epochs after the block containing their change.

#### 4. Registration Signatures (`registration-signatures`)
Generates cryptographic signatures required for Partner Chain committee candidate registration.

**Usage:**
```bash
partner-chains-node registration-signatures [OPTIONS]
```

#### 5. Address Association Signing (`sign-address-association`)
Creates signatures for associating addresses between different chains or contexts.

**Usage:**
```bash
partner-chains-node sign-address-association [OPTIONS]
```

#### 6. Block Producer Metadata Signing (`sign-block-producer-metadata`)
Signs block producer metadata for submission to the runtime, enabling participation in block production.

**Usage:**
```bash
partner-chains-node sign-block-producer-metadata [OPTIONS]
```

#### 7. Smart Contracts (`smart-contracts`)
Provides a comprehensive suite of commands for interacting with Partner Chain smart contracts deployed on Cardano.

**Usage:**
```bash
partner-chains-node smart-contracts [SUBCOMMAND]
```

#### 8. Setup Wizards (`wizards`)
Interactive text-based wizards that guide users through the setup and configuration of Partner Chains.

**Usage:**
```bash
partner-chains-node wizards [SUBCOMMAND]
```

## Architecture

### Command Structure

The crate is built around the `PartnerChainsSubcommand` enum, which defines all available commands. Each command is implemented as a separate struct with its own configuration and execution logic.

```rust
pub enum PartnerChainsSubcommand<RuntimeBindings, PartnerchainAddress> {
    SidechainParams(SidechainParamsCmd),
    RegistrationStatus(RegistrationStatusCmd),
    AriadneParameters(AriadneParametersCmd),
    // ... other commands
}
```

### Generic Design

The crate is designed with generics to support different runtime configurations and address types:

- `RuntimeBindings`: Specifies the Partner Chain runtime bindings
- `PartnerchainAddress`: Defines the address format used by the specific Partner Chain

### CLI Integration

All commands implement the `CliConfiguration` trait from Substrate's CLI framework, ensuring consistent behavior and configuration management across the Partner Chains ecosystem.

## Dependencies

### Core Dependencies

- **clap**: Command-line argument parsing
- **sc-cli**: Substrate CLI framework integration
- **sc-service**: Substrate service management
- **sp-api**: Substrate runtime API definitions
- **sp-runtime**: Substrate runtime primitives

### Partner Chains Specific

- **authority-selection-inherents**: Authority selection logic
- **cli-commands**: Core CLI command implementations
- **partner-chains-cli**: Partner Chains specific CLI utilities
- **partner-chains-smart-contracts-commands**: Smart contract interaction commands
- **sidechain-domain**: Sidechain domain types and primitives

### Cryptography and Encoding

- **parity-scale-codec**: SCALE codec for data serialization
- **sp-core**: Substrate cryptographic primitives

## Error Handling

The crate uses Substrate's error handling patterns:

- `sc_cli::Result<()>` for CLI operations
- `sc_service::Error` for service-related errors
- Custom error types for domain-specific operations

Errors are automatically converted and propagated through the CLI framework, providing clear error messages to users.

## Logging

The crate includes sophisticated logging configuration:

- **stderr**: Standard error output for general logging
- **ogmios_client.log**: Dedicated file logging for Ogmios interactions
- Configurable log levels for different components

## Development

### Building

```bash
cargo build -p partner-chains-node-commands
```

### Testing

```bash
cargo test -p partner-chains-node-commands
```

### Integration

To integrate these commands into a Partner Chain node:

1. Import the crate
2. Define your runtime bindings
3. Set up the command dependencies
4. Use the `run` function with your CLI configuration

```rust
use partner_chains_node_commands::{PartnerChainsSubcommand, run};

// In your main CLI handler
match cli.subcommand {
    Some(subcommand) => run(&cli, get_deps, subcommand),
    None => // handle default case
}
```

## Security Considerations

- All cryptographic operations use Substrate's proven primitives
- Private keys are handled securely through the CLI framework
- Command execution is isolated and validated
- Smart contract interactions include proper error handling and validation

## Contributing

When adding new commands:

1. Define the command structure with appropriate `clap` annotations
2. Implement `CliConfiguration` for consistent behavior
3. Add the command to `PartnerChainsSubcommand` enum
4. Implement execution logic in the `run` function
5. Add comprehensive tests and documentation

## License

This project is licensed under the GPL-3.0-or-later WITH Classpath-exception-2.0 license.

## See Also

- [Partner Chains Documentation](../../docs/)
- [CLI Commands Implementation](../commands/)
- [Smart Contracts Commands](../../../smart-contracts/)
- [Partner Chains CLI Utilities](../../partner-chains-cli/)
