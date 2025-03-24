# Native Token Management Pallet

## Overview

The Native Token Management pallet serves as a bridge between the main chain (Cardano) and a partner chain, providing a secure and reliable mechanism for tracking and managing native token transfers between these two ecosystems. This pallet forms a fundamental component of the cross-chain interoperability layer, enabling the seamless flow of value across chain boundaries.

At its core, this pallet solves the challenge of maintaining synchronized token supplies across two distinct blockchain environments. It achieves this by continuously monitoring events on the main chain where native tokens are sent to a designated "illiquid supply validator address," which serves as a bridge between the chains. When tokens are transferred to this address on the main chain, they are effectively locked there, and the pallet triggers the minting of an equivalent amount of tokens on the partner chain.

The pallet implements a unidirectional token transfer mechanism (from main chain to partner chain) with several key characteristics:

1. **Cross-Chain Verification**: Rather than relying on a centralized oracle, the pallet leverages an inherent data provider that connects to the main chain to verify token transfers directly from the source.

2. **Non-Custodial Design**: The locked tokens on the main chain remain in a non-custodial smart contract (the illiquid supply validator), eliminating the need for trusted intermediaries.

3. **Automatic Processing**: Token transfers are detected and processed automatically as part of the block production process through the inherent data mechanism.

4. **Transparent Configuration**: The pallet maintains clear references to the main chain scripts (policy ID, asset name, validator address) that define the native token, ensuring transparency and verifiability.

5. **Initialization Tracking**: The pallet tracks its initialization status to handle historical data appropriately, ensuring no token transfers are missed when the feature is first enabled.

This pallet is designed to be minimal yet flexible, focusing exclusively on the core functionality of tracking and processing token transfers from the main chain to the partner chain. The actual token minting logic is left to the implementing runtime, allowing for maximum flexibility in how the partner chain manages its token economy.

## Primitives

The Native Token Management pallet relies on primitives defined in the `toolkit/primitives/native-token-management` crate.

## Hooks

The Native Token Management pallet implements the following FRAME hooks to ensure proper handling of token transfers from the main chain:

### on_initialize

The `on_initialize` hook is called at the beginning of each block's execution, before any extrinsics are processed. For the Native Token Management pallet, this hook handles the verification and setup for token transfer inherent data:

```rust
fn on_initialize(n: BlockNumberFor<T>) -> Weight {
   // Function implementation
}
```

**Key responsibilities:**

1. **Inherent Data Verification Setup**: The hook establishes the verification system for token transfer inherent data to ensure:
   - When tokens are being transferred (amount > 0), the inherent must be provided
   - The token amount in the inherent matches what the main chain follower observed
   - Unexpected token transfer inherents (when no tokens are being transferred) are rejected

2. **Error Handling**: The hook ensures appropriate errors are generated when verification fails, providing clear information about what went wrong.

3. **Weight Calculation**: The hook returns an appropriate weight based on the operations performed, ensuring proper accounting of computational resources.

### on_runtime_upgrade

The pallet includes logic that runs when the runtime is upgraded:

```rust
#[pallet::hooks]
impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    // on_initialize hook
    
    // Logic that runs on runtime upgrade
    fn on_runtime_upgrade() -> Weight {
        // Migration logic
    }
}
```

**Key responsibilities:**

1. **Storage Migration**: If needed, the hook handles migration of storage from older versions of the pallet to newer ones.

2. **Initialization Handling**: Ensures that the initialization status is properly maintained across runtime upgrades.

## Configuration

The pallet uses the following configuration traits:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    
    type TokenTransferHandler: TokenTransferHandler;
    type WeightInfo: WeightInfo;
}
```

## Storage

The pallet maintains several storage items:

1. `MainChainScriptsConfiguration`: Stores the configurations needed to identify and track the native token on the main chain
2. `Initialized`: Tracks whether the pallet has been initialized, used for historical data queries

## API Specification

### Extrinsics

- **transfer_tokens**: Handles the transfer of tokens from the main chain to the partner chain
- **set_main_chain_scripts**: Updates the mainchain scripts configuration (must be called with Root origin)

### Public Functions (API)

- **get_main_chain_scripts**: Returns the current mainchain scripts configuration
- **initialized**: Returns whether the pallet has been initialized

### Inherent Data

#### Inherent Identifier
```rust
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"nattoken";
```

#### Data Type
`TokenTransferData` - Contains information about the token transfer amount from the main chain

#### Inherent Required
Yes, when token amounts greater than 0 are being transferred. The pallet verifies this inherent data to ensure tokens are properly transferred to the partner chain.

### Events

- `TokensTransfered(NativeTokenAmount)`: Emitted when tokens are transferred from the main chain to the partner chain
- `MainChainScriptsSet`: Not explicitly emitted in the code but mentioned in the README

### Errors

The pallet implements proper error handling through the inherent error type:

```rust
pub enum InherentError {
    TokenTransferNotHandled(NativeTokenAmount),
    IncorrectTokenNumberTransfered(NativeTokenAmount, NativeTokenAmount),
    UnexpectedTokenTransferInherent(NativeTokenAmount),
}
```

## Migration

See the guide in `docs/developer-guides/native-token-migration-guide.md` for how to add this
feature to an already running chain.

## Integration Example

A typical integration will include:

1. Setting up the inherent data provider in the node service:

```rust
let inherent_data_providers = sp_inherents::InherentDataProviders::new();

// Create data source that retrieves token transfer information from the main chain
let native_token_data_source = YourDataSourceImplementation::new();

// Set up the native token management inherent data provider
let native_token_inherent_provider = NativeTokenManagementInherentDataProvider::new(
    client.clone(),
    &native_token_data_source,
    latest_main_chain_hash,
    parent_hash,
).await?;

inherent_data_providers
    .register_provider(native_token_inherent_provider)
    .map_err(|e| format!("Failed to register inherent data provider: {:?}", e))?;
```

2. Implementing the `TokenTransferHandler` trait in the runtime:

```rust
pub struct NativeTokenHandler;
impl TokenTransferHandler for NativeTokenHandler {
   fn handle_token_transfer(amount: NativeTokenAmount) -> DispatchResult {
      // Chain-specific logic to mint tokens on the partner chain
      // This might involve calling functions on a currency or assets pallet
      Balances::mint(&TREASURY_ACCOUNT, amount.into())?;

      // Distribute tokens according to tokenomics rules
      distribute_tokens(amount);

      Ok(())
   }
}

impl pallet_native_token_management::Config for Runtime {
   type RuntimeEvent = RuntimeEvent;
   type TokenTransferHandler = NativeTokenHandler;
   type WeightInfo = weights::pallet_native_token_management::WeightInfo<Runtime>;
}
```

## Dependencies

- frame_system
- frame_support
- sp_runtime
- sp_inherents
- sidechain_domain (for main chain types)
