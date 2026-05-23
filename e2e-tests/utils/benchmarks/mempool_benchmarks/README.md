# Mempool Benchmarking Scripts

Scripts to extract and analyze mempool metrics from midnight-node logs, tracking transaction pool state over time.

## Overview

These scripts parse midnight-node logs to extract mempool metrics from **Ferdie's node** (the only node with detailed txpool logging enabled):

### Core Metrics
- **Ready transactions** - Valid and executable now
- **Future transactions** - Valid but waiting for dependencies (e.g., nonce)
- **Transaction count** - Total in mempool (ready + future)

### Performance Metrics
- **Validations scheduled/finished** - validated_count, revalidated
- **Submitted transactions** - submitted_count  
- **Pruned transactions** - Removed from finalized blocks
- **Reverified transactions** - Resubmitted after reorg

## Metrics Explained

### Core Metrics

**Ready Transactions**
- Valid transactions that can be executed immediately
- Have all dependencies satisfied (correct nonce, sufficient balance, etc.)
- Eligible for inclusion in the next block

**Future Transactions**  
- Valid transactions waiting for dependencies
- Typically waiting for earlier nonces or other prerequisites
- Will become "ready" once dependencies are satisfied

**Mempool Length (`mempool_len`)**
- Total number of transaction objects currently tracked
- May differ from ready + future due to internal pool management

### Activity Metrics

**Submitted Count (`submitted_count`)**
- Number of new transactions submitted to the pool in this event
- Tracks transaction admission rate

**Validated Count (`validated_count`)**
- Number of transactions validated in this event
- Transactions checked for correctness (signature, nonce, balance, etc.)

**Revalidated Count (`revalidated`)**
- Number of transactions re-validated after chain updates
- Happens when new blocks arrive and pool needs to refresh validity

**Pruned Count (`pruned`)**
- Number of transactions removed because they were included in finalized blocks
- Indicates successful transaction execution on-chain

**Reverified Count (`reverified`)**
- Number of transactions resubmitted after chain reorganization
- Happens when a fork is resolved and some transactions need to be re-added

## Log Events Tracked

The scripts extract these metrics from transaction pool events:

1. **`maintain` event** (INFO level):
   ```
   2026-01-07 12:32:55.905 INFO txpool maintain txs=(5, 2) ...
   ```
   - `txs=(ready, future)` - Current pool state

2. **`update_view_with_mempool` event** (DEBUG level):
   ```
   2026-01-07 12:32:54.150 DEBUG txpool update_view_with_mempool submitted_count=1 mempool_len=7
   ```
   - Tracks submissions and total pool size

3. **`xts_count` event** (DEBUG level):
   ```
   2026-01-07 12:32:54.150 DEBUG txpool update_view_with_mempool xts_count=(5, 2)
   ```
   - Alternative source for ready/future counts

4. **`purge_finalized_transactions` event**:
   ```
   purge_finalized_transactions count=2
   ```
   - Tracks transactions successfully included in blocks

5. **`reverified_transactions` event**:
   ```
   reverified_transactions=1
   ```
   - Tracks transactions resubmitted after reorgs

## How to Use

### Prerequisites

1. Install `python3` and `pip`
2. Install pandas: `pip install pandas matplotlib`

## Quick Start (Recommended)

Use the automated runner script to download logs and generate analysis in one command:

```bash
# With config file containing Grafana credentials
python3 run_mempool_benchmark.py \
  --config ../../../secrets/substrate/performance/performance.json \
  --from-time "2026-01-08T10:00:00Z" \
  --to-time "2026-01-08T10:10:00Z" \
  --window 1000 \
  --output-dir ./results
```

This will:
1. Download Ferdie's logs for the specified time range
2. Extract mempool metrics
3. Generate analysis with 1-second time windows
4. Save all outputs to the `results/` directory

### Manual Steps (Alternative)

1. **Download logs from Grafana/Loki** (optional)
   
   Use the download_logs.py script from the parent directory to fetch Ferdie's logs:
   ```bash
   python3 ../download_logs.py --config path/to/config.json \
     --node ferdie \
     --from-time "2026-01-07T10:00:00Z" \
     --to-time "2026-01-07T11:00:00Z" \
     --output-dir logs
   ```
   
   Note: download_logs.py is now located in the benchmarks/ directory.
   
   Or manually put Ferdie's logs in `ferdie.txt`

2. **Transform raw Grafana logs** (if needed)
   
   If you downloaded raw Grafana logs with host labels, transform them:
   ```bash
   python3 transformer.py
   ```

3. **Extract mempool data from logs**
   
   By default, processes Ferdie's logs:
   ```bash
   python3 extractor.py
   ```
   
   Or specify nodes explicitly:
   ```bash
   python3 extractor.py ferdie
   ```
   
   This generates `mempool_report.txt` with time-series data.

4. **Generate statistics and graphs**
   
   ```bash
   python3 analyzer.py mempool_report.txt analysis.txt [window_ms]
   ```
   
   Example with 1-second timeframe:
   ```bash
   python3 analyzer.py mempool_report.txt analysis.txt 1000
   ```
   
   Optional timeframe parameter (in milliseconds):
   - `100` for 100ms windows
   - `1000` for 1-second windows (default)
   - `5000` for 5-second windows

## Script Reference

### run_mempool_benchmark.py

Automated wrapper that runs all steps. Options:

```bash
python3 run_mempool_benchmark.py \
  --config <path>              # Config file with Grafana creds (optional)
  --url <url>                  # Loki URL (optional, overrides config)
  --header "Key: Value"        # Custom header (optional, repeatable)
  --from-time <iso8601>        # Start time (required)
  --to-time <iso8601>          # End time (required)
  --window <ms>                # Analysis window in ms (default: 1000)
  --output-dir <path>          # Output directory (default: .)
  --skip-download              # Skip download, use existing logs
  --skip-extract               # Skip extraction, use existing report
```

### extractor.py

Extracts metrics from log files. By default processes `ferdie.txt`:

```bash
python3 extractor.py [node1 node2 ...]
```

### analyzer.py

Analyzes extracted metrics with configurable time windows:

```bash
python3 analyzer.py <input_report> <output_analysis> [window_ms]
```

## Output

### mempool_report.txt
Time-series data showing mempool state at each logged event with columns:
- Timestamp
- Node name  
- Ready transaction count
- Future transaction count
- Mempool length
- Submitted count
- Validated count
- Revalidated count
- Pruned count
- Reverified count

### analysis.txt
Statistical summary including:
- Average ready/future transaction counts
- Peak transaction counts
- Admission rates (TPS) over different time windows
- Total validated/revalidated transactions
- Total pruned transactions
- Total reverified (resubmitted) transactions

## Example Output

```
=== MEMPOOL STATISTICS BY NODE ===

Node: ferdie
  Average Ready Txs: 12.5
  Average Future Txs: 3.2
  Peak Ready Txs: 45
  Peak Future Txs: 15
  Avg Admission Rate: 8.3 TPS
  Total Validated: 1523
  Total Revalidated: 342
  Total Pruned: 1489
  Total Reverified: 28
```
