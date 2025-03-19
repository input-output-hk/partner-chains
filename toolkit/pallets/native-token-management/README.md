# Native Token Management Pallet

## Overview

The Native Token Management pallet serves as a bridge between a main chain (such as Cardano) and a partner chain, providing a secure and reliable mechanism for tracking and managing native token transfers between these two ecosystems. This pallet forms a fundamental component of the cross-chain interoperability layer, enabling the seamless flow of value across chain boundaries.

At its core, this pallet solves the challenge of maintaining synchronized token supplies across two distinct blockchain environments. It achieves this by continuously monitoring events on the main chain where native tokens are sent to a designated "illiquid supply validator address," which serves as a bridge between the chains. When tokens are transferred to this address on the main chain, they are effectively locked there, and the pallet triggers the minting of an equivalent amount of tokens on the partner chain.

The pallet implements a unidirectional token transfer mechanism (from main chain to partner chain) with several key characteristics:

1. **Cross-Chain Verification**: Rather than relying on a centralized oracle, the pallet leverages an inherent data provider that connects to the main chain to verify token transfers directly from the source.

2. **Non-Custodial Design**: The locked tokens on the main chain remain in a non-custodial smart contract (the illiquid supply validator), eliminating the need for trusted intermediaries.

3. **Automatic Processing**: Token transfers are detected and processed automatically as part of the block production process through the inherent data mechanism.

4. **Transparent Configuration**: The pallet maintains clear references to the main chain scripts (policy ID, asset name, validator address) that define the native token, ensuring transparency and verifiability.

5. **Initialization Tracking**: The pallet tracks its initialization status to handle historical data appropriately, ensuring no token transfers are missed when the feature is first enabled.

This pallet is designed to be minimal yet flexible, focusing exclusively on the core functionality of tracking and processing token transfers from the main chain to the partner chain. The actual token minting logic is left to the implementing runtime, allowing for maximum flexibility in how the partner chain manages its token economy.

## Primitives

The Native Token Management pallet relies on primitives defined in the `toolkit/primitives/native-token-management` crate:

### Main Chain Scripts

The key data structure used by the pallet is `MainChainScripts`, which identifies on-chain entities involved in the native token management system:

```rust
pub struct MainChainScripts {
    /// Minting policy ID of the native token
    pub native_token_policy_id: PolicyId,
    /// Asset name of the native token
    pub native_token_asset_name: AssetName,
    /// Address of the illiquid supply validator. All tokens sent to that address are effectively locked
    /// and considered "sent" to the Partner Chain.
    pub illiquid_supply_validator_address: MainchainAddress,
}
```

This structure includes helpful methods for standard library environments:

```rust
impl MainChainScripts {
    pub fn read_from_env() -> Result<Self, envy::Error>
}
```

### Inherent Data Handling

1. **INHERENT_IDENTIFIER**: Used to identify native token transfer inherent data
   ```rust
   pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"nattoken";
   ```

2. **TokenTransferData**: Represents token transfer data for inherent processing
   ```rust
   pub struct TokenTransferData {
       pub token_amount: NativeTokenAmount,
   }
   ```

3. **InherentError**: Defines errors that can occur during inherent data processing
   ```rust
   pub enum InherentError {
       TokenTransferNotHandled(NativeTokenAmount),
       IncorrectTokenNumberTransfered(NativeTokenAmount, NativeTokenAmount),
       UnexpectedTokenTransferInherent(NativeTokenAmount),
   }
   ```

### Runtime API

The primitives define a runtime API for managing native tokens:

```rust
pub trait NativeTokenManagementApi {
    fn get_main_chain_scripts() -> Option<MainChainScripts>;
    /// Gets current initializaion status and set it to `true` afterwards. This check is used to
    /// determine whether historical data from the beginning of main chain should be queried.
    fn initialized() -> bool;
}
```

### Inherent Data Provider

For runtimes that support the standard library, the primitives provide an inherent data provider:

```rust
pub struct NativeTokenManagementInherentDataProvider {
    pub token_amount: Option<NativeTokenAmount>,
}
```

This provider interfaces with a data source that retrieves token transfer information from the main chain:

```rust
pub trait NativeTokenManagementDataSource {
    /// Retrieves total of native token transfers into the illiquid supply in the range (after_block, to_block]
    async fn get_total_native_token_transfer(
        &self,
        after_block: Option<McBlockHash>,
        to_block: McBlockHash,
        scripts: MainChainScripts,
    ) -> Result<NativeTokenAmount, Box<dyn std::error::Error + Send + Sync>>;
}
```

The inherent data provider includes a constructor method that automatically detects whether the pallet is present in the runtime:

```rust
impl NativeTokenManagementInherentDataProvider {
    /// Creates inherent data provider only if the pallet is present in the runtime.
    /// Returns zero transfers if not.
    pub async fn new<Block, C>(
        client: Arc<C>,
        data_source: &(dyn NativeTokenManagementDataSource + Send + Sync),
        mc_hash: McBlockHash,
        parent_hash: <Block as BlockT>::Hash,
    ) -> Result<Self, IDPCreationError>
}
```

## Configuration

The pallet uses the following configuration traits:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// Handler for transferring tokens from native environment to the partner chain.
    /// This is called when native token lock event is detected on the main chain.
    type TokenHandler: TokenHandler;
}
```

## Storage

The pallet maintains several storage items:

1. `MainChainScripts`: Stores the configurations needed to identify and track the native token on the main chain
2. `Initialized`: Tracks whether the pallet has been initialized, used for historical data queries

## API Specification

### Extrinsics

#### transfer_tokens
Handles the transfer of tokens from the main chain to the partner chain

```rust
fn transfer_tokens(
    origin: OriginFor<T>,
    token_amount: NativeTokenAmount,
) -> DispatchResultWithPostInfo
```

Parameters:
- `token_amount`: The amount of tokens to transfer

#### set_main_chain_scripts
Updates the mainchain scripts configuration

```rust
fn set_main_chain_scripts(
    origin: OriginFor<T>,
    native_token_policy_id: PolicyId,
    native_token_asset_name: AssetName,
    illiquid_supply_validator_address: MainchainAddress,
) -> DispatchResultWithPostInfo
```

Parameters:
- `native_token_policy_id`: The policy ID of the native token
- `native_token_asset_name`: The asset name of the native token
- `illiquid_supply_validator_address`: The address of the validator handling illiquid supply

### Public Functions (API)

#### get_main_chain_scripts
Returns the current mainchain scripts configuration

```rust
fn get_main_chain_scripts() -> Option<MainChainScripts>
```

Returns:
- `Option<MainChainScripts>`: The current mainchain scripts if configured, or None

#### initialized
Returns whether the pallet has been initialized and marks it as initialized if not

```rust
fn initialized() -> bool
```

Returns:
- `bool`: Whether the pallet was already initialized before this call

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

- `TokensTransferred(NativeTokenAmount)`: Emitted when tokens are transferred from the main chain to the partner chain
- `MainChainScriptsSet`: Emitted when the mainchain scripts configuration is updated

### Errors

- `MainChainScriptsNotConfigured`: The required mainchain scripts have not been configured
- `ZeroTokenAmount`: Attempted to transfer zero tokens, which is not allowed

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

2. Implementing the `TokenHandler` trait in the runtime:

```rust
pub struct NativeTokenHandler;
impl TokenHandler for NativeTokenHandler {
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
    type TokenHandler = NativeTokenHandler;
}
```

## Dependencies

- frame_system
- frame_support
- sp_runtime
- sp_inherents
- sidechain_domain (for main chain types)
