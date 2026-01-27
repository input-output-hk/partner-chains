# Transaction Benchmark Setup Guide

This guide walks through the complete process of generating and submitting transactions on a new environment or network. Follow these steps in order to set up your test wallets and execute benchmark transactions.

## Prerequisites

Before starting, ensure you have:

1. **Midnight Node Toolkit** installed and accessible in your PATH
   - The scripts expect the command `midnight-node-toolkit` to be available
   - Verify installation: `midnight-node-toolkit --version`

2. **toolkit.db file** in the current directory
   - This database file is required for wallet operations
   - If it's located elsewhere, you'll be prompted to provide the path

3. **Python 3** with standard libraries (no additional packages required)

4. **Network Configuration** for your target environment
   - Node URLs and relay names
   - You'll need to update the `RELAYS` list in each script (see Configuration section)

## Configuration

Before running the scripts, you **must** configure them for your target environment. The following configuration values need to be set in each Python script:

### 1. Update Relay Node List

**Required for all scripts.** Open each Python file and modify the `RELAYS` list near the top to match your network's relay nodes:

```python
RELAYS = [
    "ferdie",
    "george",
    "henry",
    "iris",
    "jack",
    "paul",
    "quinn",
    "rita",
    "sam",
    "tom"
]
```

**Files to update:**
- `register_dust.py`
- `fund_wallets.py`
- `get_balances.py`
- `generate_txs_round_robin.py`
- `send_batch_txs.py`

### 2. Update Node URL Pattern

The relay names are used to construct WebSocket URLs in the format:
```python
node_url = f"ws://{relay_name}.node.sc.iog.io:9944"
```

If your environment uses a different URL pattern (domain, port, or protocol), you'll need to update the URL construction in each script:
- In `register_dust.py`, line 136
- In `fund_wallets.py`, line 158
- In `get_balances.py`, line 36
- In `generate_txs_round_robin.py`, line 84
- In `send_batch_txs.py`, line 35

### 3. Verify Token Configuration

The scripts are configured for the native token by default:

```python
TOKEN_TYPE = "0000000000000000000000000000000000000000000000000000000000000000"
```

This is found in:
- `fund_wallets.py`, line 32
- `generate_txs_round_robin.py`, line 27

If you need to use a different token type, update this value in the relevant scripts.

### 4. Adjust Transaction Amount (Optional)

The round-robin transactions use a base amount with randomization to avoid collisions:

```python
BASE_AMOUNT = 1000000  # In smallest token unit
```

Transactions will send `BASE_AMOUNT ± 100` (randomized). This is defined in `generate_txs_round_robin.py`, line 28.

### 5. Command-Line Configurable Defaults

Each script has configurable defaults that can be overridden with command-line arguments:

- **Wallet Index Range**: Which seed indices to use (e.g., 10-100)
- **Funding Account Range**: Which accounts provide funding (e.g., 1-3)
- **Token Amount**: How much to fund per wallet (only in `fund_wallets.py`)
- **Number of Workers**: Concurrency level (only in `send_batch_txs.py`)

## Step-by-Step Workflow

### Step 0: Set Up Funding Accounts

**Before you begin**, you need funding accounts with sufficient balance to pay for:
1. Dust registrations for all test wallets
2. Initial funding for all test wallets

The scripts use **funding accounts** (typically seed indices 1-3) as the source of funds. These must be pre-funded before you can proceed.

#### Option A: Use a Faucet (If Available)

If your network has a faucet service, use it to fund your funding accounts:

1. Generate addresses for your funding accounts:
   ```bash
   midnight-node-toolkit show-address --network undeployed --seed 0000000000000000000000000000000000000000000000000000000000000001
   midnight-node-toolkit show-address --network undeployed --seed 0000000000000000000000000000000000000000000000000000000000000002
   midnight-node-toolkit show-address --network undeployed --seed 0000000000000000000000000000000000000000000000000000000000000003
   ```

2. Submit these addresses to your network's faucet to receive tokens

3. Wait for the funding transactions to be finalized

#### Option B: Transfer from Existing Accounts

If you already have funded accounts on the network, transfer funds to your funding accounts (seeds 1-3) using the toolkit or your preferred method.

#### Verify Funding Accounts Have Sufficient Balance

Once funded, verify your funding accounts have sufficient tokens:

```bash
python3 get_balances.py --start 1 --end 3
```

**How much do you need?**
- For dust registration: Small amount per wallet (typically < 1 token per registration)
- For wallet funding: Amount specified by `--night-amount` × number of test wallets
- Example: For 100 wallets with 3M tokens each, you need ~300M tokens total plus overhead

### Step 1: Register Dust Addresses

**IMPORTANT**: This must be the **first** step. Dust registration must be done **before** funding the wallets. The registration transaction is paid for by the funding accounts.

**Command:**
```bash
python3 register_dust.py --dest-start 10 --dest-end 100 --fund-start 1 --fund-end 3
```

**Parameters:**
- `--start 10`: Start with seed index 10
- `--end 100`: End with seed index 100
- `--funding-start 1`: First funding account (pays for registration)
- `--funding-end 3`: Last funding account (distributes load)
- `--verbose`: (Optional) Enable detailed command output for debugging

**What it does:**
- Registers dust addresses for all wallets in the specified range
- Each registration is funded by one of the funding accounts
- Uses round-robin distribution across relay nodes
- Waits 2 seconds between registrations to avoid nonce conflicts

**Output example:**
```
🚀 Starting dust registration for seeds ending in 10 to 100...
ℹ️  Using 3 threads for execution.
🚀 Starting chunk 10-40 on ferdie with funding seed ...01
[Chunk 01] Registering dust for seed ...10...
✅ Success (Seed ...10)
...
🎉 All registration commands completed.
⏱️ Total execution time for 91 wallets: 4 minutes and 12.67 seconds
📊 Average time per registration: 2.78 seconds
```

**Common Issues:**
- If you see "len (is 0)" error: The funding wallet has no funds. Make sure funding accounts are funded first!
- If registrations fail: Verify the funding accounts have sufficient balance

### Step 2: Fund Wallets

After dust registration, fund the test wallets that will be used for transactions. This script generates wallet addresses and sends tokens from your funding accounts.

**Command:**
```bash
python3 fund_wallets.py --start 10 --end 100 --funding-start 1 --funding-end 3 --night-amount 3000000
```

**Parameters:**
- `--start 10`: Start with seed index 10
- `--end 100`: End with seed index 100 (creates 91 wallets)
- `--funding-start 1`: First funding account is seed index 1
- `--funding-end 3`: Last funding account is seed index 3 (uses 3 funding accounts)
- `--night-amount 3000000`: Send 3,000,000 NIGHT tokens per wallet (automatically converted to smallest unit)

**What it does:**
- Generates wallet addresses for seeds 10-100
- Distributes funding work across multiple funding accounts and relay nodes
- Uses multi-threading for parallel execution
- Waits 2 seconds between transactions to ensure proper nonce propagation

**Output example:**
```
🚀 Starting wallet creation and funding script...
ℹ️  Using 3 threads for execution.
🚀 Starting chunk 10-40 on ferdie
[Chunk 0001] Generating wallet 10... ✅ 0x1234...
[Chunk 0001] Funding 0x1234... ✅ Sent
...
🎉 All operations completed successfully.
⏱️ Total execution time for 91 wallets: 3 minutes and 45.23 seconds
📊 Average time per funding: 2.48 seconds
```

### Step 3: Verify Wallet Balances

Before generating transactions, verify that your wallets were funded correctly.

**Command:**
```bash
python3 get_balances.py --start 10 --end 100
```

**Parameters:**
- `--start 10`: First wallet to check
- `--end 100`: Last wallet to check

**What it does:**
- Queries the balance for each wallet in parallel
- Displays individual balances and timing information
- Shows total balance across all wallets

**Output example:**
```
🚀 Checking balances for seeds 10 to 100 across 10 nodes...
ℹ️  Using 8 threads.
Seed 10: 3000000000000 [DB Copy: 0.0234s, Exec: 1.2341s]
Seed 11: 3000000000000 [DB Copy: 0.0198s, Exec: 1.1892s]
...
💰 Total Balance: 273000000000000
⏱️ Total execution time: 24.56 seconds
```

**Note:** Balances are shown in the smallest token unit (e.g., 3000000000000 = 3,000,000 NIGHT tokens with 6 decimal places).

### Step 4: Generate Transactions

**What transactions are generated:**

The scripts create **round-robin transactions** in a ring topology. Each wallet sends a small transaction to the next wallet in sequence, and the last wallet sends back to the first, creating a continuous loop:

```
Wallet 10 → Wallet 11 → Wallet 12 → ... → Wallet 100 → Wallet 10 (loops back)
```

**Transaction characteristics:**
- **Amount**: Small transactions (default: 1,000,000 in smallest token unit, with ±100 randomization)
- **Purpose**: Generate continuous transaction load for benchmarking network throughput
- **Distribution**: Transactions are distributed across all relay nodes in the `RELAYS` list using round-robin selection
- **Token Type**: Native token (configurable in the script)

This pattern ensures:
- All wallets remain funded (they both send and receive)
- Unique transaction amounts (via randomization) to avoid collisions
- Even distribution of load across relay nodes
- Continuous transaction flow for sustained benchmarking

**Command (Generate and Save to Files):**
```bash
python3 generate_txs_round_robin.py --start 10 --end 100
```

**Command (Generate and Submit Immediately):**
```bash
python3 generate_txs_round_robin.py --start 10 --end 100 --submit
```

**Parameters:**
- `--start 10`: First wallet in the round-robin ring
- `--end 100`: Last wallet in the round-robin ring
- `--submit`: (Optional) Submit transactions directly instead of saving to files
- `--verbose`: (Optional) Show detailed toolkit command output

**What it does:**
- Creates a ring topology: each wallet sends to the next one
- Randomizes transaction amounts (BASE_AMOUNT ± 100) to avoid collisions
- Saves transactions to `txs/tx_<timestamp>_<index>.mn` files (unless `--submit` is used)
- Uses round-robin relay node selection for better distribution
- Multi-threaded execution for parallel transaction generation

**Output example:**
```
🚀 Starting ring transaction script (10 -> 11 -> ... -> 100 -> 10)...
ℹ️  Using 8 threads for execution.
Processing: Seed 10 -> Seed 11 (Amount: 1000034)...
✅ Seed 10 -> Seed 11 Saved (1000034) [DB Copy: 0.0212s, Exec: 2.3451s]
Processing: Seed 11 -> Seed 12 (Amount: 999978)...
✅ Seed 11 -> Seed 12 Saved (999978) [DB Copy: 0.0198s, Exec: 2.2876s]
...
🎉 Batch processing complete.
Valid: 91, Invalid: 0
⏱️ Total execution time: 187.34 seconds
```

**Generated Files:**
Transaction files are saved in the `txs/` directory:
```
txs/
├── tx_1737908123_10.mn
├── tx_1737908125_11.mn
├── tx_1737908127_12.mn
└── ...
```

### Step 5: Submit Batch Transactions

Submit the pre-generated transaction files to the network.

**Command (Submit All Transactions):**
```bash
python3 send_batch_txs.py
```

**Command (Submit Specific Range):**
```bash
python3 send_batch_txs.py --start 10 --end 50
```

**Command (Control Concurrency):**
```bash
python3 send_batch_txs.py --workers 5 --verbose
```

**Parameters:**
- `--start 10`: (Optional) Only submit transactions from seed index 10 onwards
- `--end 50`: (Optional) Only submit transactions up to seed index 50
- `--workers 5`: (Optional) Number of concurrent submission threads (default: CPU count)
- `--verbose`: (Optional) Show detailed command output
- `--batch-size 20`: (Optional) Number of transactions to submit per batch (default: 0, submit all at once).
- `--batch-delay 6.0`: (Optional) Delay in seconds between batches (default: 6.0s).

**What it does:**
- Scans the `txs/` directory for transaction files matching `tx_*.mn`
- Submits each transaction to relay nodes with automatic retry on failure
- Uses round-robin relay selection for load distribution
- Tracks chain latency (time from SENT to FINALIZED)
- Handles common errors (Invalid Transaction 1010, Temporarily Banned 1012)
- Adds small random delays between submissions when using multiple workers

**Output example:**
```
🚀 Found 91 transaction files to submit.
ℹ️  Using 8 threads for execution.
✅ [1/91] Sent txs/tx_1737908123_10.mn to ferdie [Chain Latency: 3.42s, Exec: 4.1234s]
✅ [2/91] Sent txs/tx_1737908125_11.mn to george [Chain Latency: 3.28s, Exec: 3.9876s]
⚠️  txs/tx_1737908127_12.mn: {"timestamp":1737908130456,"message":"SENT"}
✅ [3/91] Sent txs/tx_1737908127_12.mn to henry [Chain Latency: 3.15s, Exec: 4.0123s]
...
🎉 Batch submission complete.
Valid: 89, Invalid: 1, Temporarily Banned: 1
⏱️ Total execution time: 234.67 seconds
```

**Common Issues:**
- **Invalid Transaction (1010)**: Transaction is invalid (e.g., insufficient funds, invalid nonce)
- **Temporarily Banned (1012)**: Transaction was already submitted or conflicts with another transaction
- **Retry on different nodes**: If a relay fails, the script automatically tries other nodes

### Step 6 (Optional): Count Validated Transactions

After submitting transactions, use `tx-counter.py` to verify how many transactions were successfully validated by the network.

**What it does:**
- Parses node logs to find "Validated Midnight transaction" entries
- Counts distinct transactions by hash (deduplicated across all nodes)
- Reports per-node counts and total validated transactions
- **Important**: This counts validated transactions (confirmed by nodes), not just submissions

**Three modes of operation:**

#### 1. Download logs and count (recommended for live networks)

```bash
python3 tx-counter.py --download \
  --from-time "2026-01-26T18:00:00Z" \
  --to-time "2026-01-26T18:15:00Z" \
  --node alice --node bob --node charlie \
  --config ../../../secrets/substrate/performance/performance.json
```

**Parameters:**
- `--download`: Enable download mode
- `--from-time`: Start of time window (ISO 8601 format)
- `--to-time`: End of time window (ISO 8601 format)
- `--node`: Node names to download logs from (can specify multiple times)
- `--config`: Path to encrypted config file with Grafana/Loki credentials
- `--url`: (Optional) Override Grafana/Loki URL
- `--header`: (Optional) Add authorization header (e.g., `"Authorization: Bearer TOKEN"`)

Alternatively, use `--time-range` with JSON:
```bash
python3 tx-counter.py --download \
  --time-range '{"from":"2026-01-26 18:00:00","to":"2026-01-26 18:15:00"}' \
  --node alice --node bob
```

#### 2. Count from existing log directory

```bash
python3 tx-counter.py --log-dir ../logs/from_2026-01-26_18-00-00_to_2026-01-26_18-15-00
```

#### 3. Count from single log file

```bash
python3 tx-counter.py path/to/node.txt
```

**Output example:**
```
Processing 3 log files...

alice: 89 distinct transactions
bob: 87 distinct transactions
charlie: 91 distinct transactions

==================================================
TOTAL DISTINCT TRANSACTIONS ACROSS ALL NODES: 91
==================================================

First 5 transaction hashes found:
 - 618804a1b2c3d4e5f6...
 - 7289015b3c4d5e6f7a...
 - 839126c4d5e6f7a8b9...
 - 94a237d5e6f7a8b9ca...
 - a5b348e6f7a8b9cadb...
```

**Tips:**
- Ensure your time window covers when you submitted transactions
- Counts may be less than submissions if some transactions were invalid or rejected
- This script does not require `toolkit.db`
- Logs are downloaded to `../logs/from_<start>_to_<end>/` directory

## Advanced Usage

### Custom Wallet Ranges

You can work with different wallet ranges for different purposes:

```bash
# Create 500 wallets for high-volume testing
# First, register dust
python3 register_dust.py --start 10 --end 509 --funding-start 1 --funding-end 5

# Then fund them
python3 fund_wallets.py --start 10 --end 509 --funding-start 1 --funding-end 5

# Generate transactions
python3 generate_txs_round_robin.py --start 10 --end 509
```

### Partial Submission

Submit transactions in batches to control load:

```bash
# Submit first 50 transactions
python3 send_batch_txs.py --start 10 --end 59

# Wait for processing, then submit next batch
python3 send_batch_txs.py --start 60 --end 109

# And so on...
```

### Debugging

Enable verbose output to see detailed toolkit commands and responses:

```bash
python3 register_dust.py --start 10 --end 20 --verbose
python3 generate_txs_round_robin.py --start 10 --end 20 --verbose
python3 send_batch_txs.py --start 10 --end 20 --verbose
```

### Controlling Concurrency

Adjust the number of parallel workers based on your system and network:

```bash
# Use fewer workers to reduce network load
python3 send_batch_txs.py --workers 3

# Let the system decide (default: CPU count)
python3 send_batch_txs.py
```

## Important Notes

### Seed Format

All wallets use 64-character zero-padded seed indices:
- Seed index `10` becomes `0000000000000000000000000000000000000000000000000000000000000010`
- Seed index `100` becomes `0000000000000000000000000000000000000000000000000000000000000100`

### Database Locking

Scripts automatically create temporary copies of `toolkit.db` to avoid database locking issues when running multi-threaded operations. You don't need to manage this manually.

### Transaction Amounts

- **Funding**: Configurable via `--night-amount` (in NIGHT tokens, automatically converted to smallest unit)
- **Transactions**: Base amount of 1,000,000 (smallest unit) with randomization ±100 to avoid collisions
- **Token Type**: Default is native token (64 zeros)

### Timing Considerations

- **2-second delays** are built into funding and registration to ensure proper nonce propagation
- **Random delays** (0.05-0.5s) are added during batch submission when using multiple workers
- These can be adjusted in the script source if needed for your environment

### Error Handling

Scripts continue processing even if individual operations fail. Check the output summary to see how many operations succeeded vs. failed.

### Round-Robin Distribution

All scripts use round-robin distribution across:
- **Relay nodes**: To balance network load
- **Funding accounts**: To parallelize operations and avoid nonce conflicts
- **Worker threads**: To maximize parallelism

## Troubleshooting

### "Could not find 'midnight-node-toolkit'"

Ensure the toolkit is installed and in your PATH:
```bash
which midnight-node-toolkit
```

### "toolkit.db not found"

The scripts need access to `toolkit.db`. Either:
1. Run scripts from the directory containing `toolkit.db`, or
2. Provide the path when prompted

### Funding Account Has No Funds

If you see errors like "len (is 0)" during registration:
1. Check your funding account balances
2. Ensure funding accounts (seeds 1-3 by default) are properly funded
3. Use `get_balances.py` to verify: `python3 get_balances.py --start 1 --end 3`

### Nonce/Sequence Errors

These usually occur when operations are too fast:
- The scripts include built-in delays to prevent this
- If you still see errors, you may need to increase delays in the source code
- Reduce the number of concurrent workers: `--workers 2`

### Network Connectivity Issues

If transactions fail to submit:
1. Verify relay node URLs are correct and accessible
2. Check WebSocket connectivity: `wscat -c ws://ferdie.node.sc.iog.io:9944`
3. Try with fewer concurrent workers to reduce network load

## Quick Reference

### Complete Workflow Commands

```bash
# 0. Verify funding accounts have sufficient funds (required for steps 1 and 2)
python3 get_balances.py --start 1 --end 3

# 1. Register dust addresses (MUST be done FIRST, before funding wallets)
python3 register_dust.py --start 10 --end 100 --funding-start 1 --funding-end 3

# 2. Fund wallets (creates and funds 91 wallets)
python3 fund_wallets.py --start 10 --end 100 --funding-start 1 --funding-end 3

# 3. Verify funding was successful
python3 get_balances.py --start 10 --end 100

# 4. Generate transactions (saves to txs/ directory)
python3 generate_txs_round_robin.py --start 10 --end 100

# 5. Submit transactions to network
python3 send_batch_txs.py

# 6. (Optional) Count validated transactions from logs
python3 tx-counter.py --download \
  --from-time "2026-01-26T18:00:00Z" \
  --to-time "2026-01-26T18:15:00Z" \
  --node alice --node bob \
  --config ../../../secrets/substrate/performance/performance.json

# OR: Skip step 4 and submit directly
python3 generate_txs_round_robin.py --start 10 --end 100 --submit
```

### File Locations

- **Scripts**: Current directory
- **Transaction files**: `txs/tx_<timestamp>_<index>.mn`
- **Database**: `toolkit.db` (in current directory or custom path)

### Environment Variables

- `MN_DONT_WATCH_PROGRESS`: Set to `"false"` during funding/registration, `"true"` during submission (handled automatically by scripts)
