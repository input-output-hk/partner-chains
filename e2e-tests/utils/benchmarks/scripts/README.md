# Transaction Utilities

This directory contains shared utilities for creating, funding, and submitting transactions during benchmark runs.

## Scripts

### fund_wallets.py

Funds test wallets with tokens from source accounts.

**Usage:**
```bash
python3 fund_wallets.py --start 10 --end 99
```

**Options:**
- `--start` - Starting seed index (default: 10)
- `--end` - Ending seed index (default: 99)

### register_dust.py

Registers dust addresses for test wallets.

**Usage:**
```bash
python3 register_dust.py --start 10 --end 99
```

**Options:**
- `--start` - Starting seed index (default: 10)
- `--end` - Ending seed index (default: 99)

### generate_txs_round_robin.py

Generates round-robin transactions between wallets (each wallet sends to the next in sequence).

**Usage:**
```bash
# Generate and save to files
python3 generate_txs_round_robin.py --start 10 --end 499

# Submit directly
python3 generate_txs_round_robin.py --submit --start 10 --end 499

# Verbose mode
python3 generate_txs_round_robin.py --verbose --start 10 --end 499
```

**Options:**
- `--submit` - Submit transactions directly instead of saving to file
- `--verbose` - Enable verbose output
- `--start` - Starting seed index (default: 10)
- `--end` - Ending seed index (default: 499)

### send_batch_txs.py

Submits pre-generated transaction files in batches.

**Usage:**
```bash
# Submit all transactions in txs/ directory
python3 send_batch_txs.py

# Submit specific range
python3 send_batch_txs.py --start 10 --end 100

# Control concurrency
python3 send_batch_txs.py --workers 5 --verbose
```

**Options:**
- `--start` - Start index for transaction files
- `--end` - End index for transaction files
- `--verbose` - Enable verbose output
- `--workers` - Number of concurrent workers

### send_txs_round_robin.py

Submits round-robin transactions directly (simplified version).

**Usage:**
```bash
python3 send_txs_round_robin.py
```

Configuration is defined in the script constants (START_INDEX, END_INDEX, etc.).

### tx-counter.py

Counts distinct validated transactions in log files.

**Usage:**
```bash
# Download logs and count transactions
python3 tx-counter.py --download \
  --config ../../../secrets/substrate/performance/performance.json \
  --from-time "2026-01-14T05:37:00Z" \
  --to-time "2026-01-14T06:00:00Z" \
  --node alice --node bob --node charlie

# Count transactions in existing log directory
python3 tx-counter.py --log-dir ../logs/from_2026-01-14_05-37-00_to_2026-01-14_06-00-00

# Count transactions in a single log file
python3 tx-counter.py path/to/node.txt
```

**Options:**
- `--download` - Download logs before counting
- `--from-time` - Start time (ISO 8601)
- `--to-time` - End time (ISO 8601)
- `--node` - Node to download (can be used multiple times)
- `--config` - Path to encrypted config file
- `--header` - Authorization header (can be used multiple times)
- `--url` - Grafana/Loki URL
- `--log-dir` - Path to existing log directory

### deploy_mint_contract.sh

Deploys a minting contract.

**Note:** In order to deploy a minting contract, you have to use `toolkit-js` from the `midnight-node` GitHub repository.

This script performs the following actions:
1. Builds the compact contract from the `midnight-node` repository.
2. Generates contract-specific files in the `toolkit-js` directory:
   - In `mint/`: Generates `mint.config.js` from `mint.compact` and `mint.config.ts`.
   - In `mint/out/`: Creates `compiler`, `contract`, `keys`, and `zkir` directories with associated files.
3. Generates transaction and state files in the working directory (`WORK_DIR`):
   - `contract_state.mn`, `deploy_tx.mn`, `deploy.bin`, `deploy_zswap.json`
   - `mint.bin`, `mint_zswap.json`
   - `private_state.json`, `private_state2.json`
All these output are necessary for calling the mint contract.

**Usage:**
```bash
./deploy_mint_contract.sh
```

### call_mint_contract.py

Calls a deployed minting contract.

**Note:** Run the deployment script first. You must obtain the files created at `midnight-node/util/toolkit-js/mint` and WORK_DIR to use them with this script.

**Usage:**
```bash
python3 call_mint_contract.py --toolkit-js-path /path/to/midnight-node/util/toolkit-js
```

## Prerequisites

1. Install the Midnight Node Toolkit:
   ```bash
   # Follow instructions from the Midnight Node documentation
   ```

2. Ensure `toolkit.db` exists in the working directory (or provide path when prompted)

3. For scripts that download logs, ensure Grafana credentials are configured

## Common Workflow

1. Fund wallets:
   ```bash
   python3 fund_wallets.py --start 10 --end 100
   ```

2. Register dust addresses:
   ```bash
   python3 register_dust.py --start 10 --end 100
   ```

3. Generate transactions:
   ```bash
   python3 generate_txs_round_robin.py --start 10 --end 100
   ```

4. Submit transactions:
   ```bash
   python3 send_batch_txs.py --start 10 --end 100
   ```

5. Count validated transactions:
   ```bash
   python3 tx-counter.py --download \
     --config ../../../secrets/substrate/performance/performance.json \
     --from-time "2026-01-14T05:37:00Z" \
     --to-time "2026-01-14T06:00:00Z" \
     --node alice --node bob
   ```

## Notes

- All scripts use multi-threading for parallel execution
- Scripts automatically distribute work across multiple source accounts when available
- Transaction amounts include randomization to avoid collisions
- Scripts use round-robin relay node selection for better load distribution
- The toolkit.db file is copied to temporary directories to avoid database locking issues
