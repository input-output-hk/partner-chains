# Partner Chains Toolkit Pallets

This directory contains the core pallets (modules) that make up the Partner Chains Toolkit. Each pallet is responsible for a specific aspect of functionality in the partner chain ecosystem.

## Pallet Overview

### 📚 [address-associations](./address-associations)
Manages associations between different address types across different chains and systems.

### 📊 [block-participation](./block-participation)
Tracks validator participation in block production and maintains records of validator activity.

### 📝 [block-production-log](./block-production-log)
Maintains a chronological record of which validators have produced blocks at specific slots throughout the blockchain's lifetime.

### 💰 [native-token-management](./native-token-management)
Handles the management of the native token including issuance, transfers, and other related operations.

### 👥 [partner-chains-session](./partner-chains-session)
Manages session-related functionality for partner chains, including validator set management across sessions.

### 🔐 [session-validator-management](./session-validator-management)
Provides tools for managing validators within a session, including selection, rotation, and oversight.

### 🌉 [sidechain](./sidechain)
Implements core functionality for sidechains connecting to the main network.
