# Partner Chains Benchmarking Tools

This directory contains tools for benchmarking and analyzing Partner Chains node performance.

## Directory Structure

```
benchmarks/
├── README.md                    # This file
├── download_logs.py             # Shared utility for downloading logs from Grafana/Loki
├── block_size_benchmarks/       # Block propagation and size benchmarking
├── mempool_benchmarks/          # Mempool transaction pool benchmarking
└── utils/                       # Shared transaction utilities
```

## Overview

### download_logs.py

Shared utility script for downloading logs from Grafana/Loki. Used by all benchmark scripts.

**Usage:**
```bash
python3 download_logs.py \
  --config ../../secrets/substrate/performance/performance.json \
  --from-time "2026-01-07T10:00:00Z" \
  --to-time "2026-01-07T10:10:00Z" \
  --node alice --node bob
```

Logs are downloaded to `benchmarks/logs/from_YYYY-MM-DD_HH-MM-SS_to_YYYY-MM-DD_HH-MM-SS/`

### block_size_benchmarks/

Tools for measuring block propagation times and analyzing block creation performance across the network.

Key files:
- `run_benchmark.py` - Main automated workflow script
- `extractor.py` - Extract propagation data from logs
- `analyzer.py` - Generate statistics and analysis
- `README.md` - Detailed usage instructions

See [block_size_benchmarks/README.md](block_size_benchmarks/README.md) for details.

### mempool_benchmarks/

Tools for analyzing mempool transaction pool metrics including ready/future transactions, validation rates, and admission rates.

Key files:
- `run_mempool_benchmark.py` - Main automated workflow script
- `extractor.py` - Extract mempool metrics from logs
- `analyzer.py` - Generate statistics and graphs
- `README.md` - Detailed usage instructions

See [mempool_benchmarks/README.md](mempool_benchmarks/README.md) for details.

### utils/

Shared transaction utilities for creating, funding, and submitting transactions during benchmark runs.

Key scripts:
- `fund_wallets.py` - Fund test wallets with tokens
- `register_dust.py` - Register dust addresses
- `generate_txs_round_robin.py` - Generate round-robin transactions
- `send_batch_txs.py` - Submit transactions in batches
- `send_txs_round_robin.py` - Submit round-robin transactions
- `tx-counter.py` - Count validated transactions in logs

These utilities are used by benchmark scripts to set up test scenarios and analyze results.

## Prerequisites

1. Install Python 3 and required packages:
   ```bash
   pip install pandas requests matplotlib
   ```

2. Install `sops` for encrypted config files:
   ```bash
   brew install sops
   ```

3. Set up Grafana access (see block_size_benchmarks/README.md for details)

## Quick Start

### Block Size Benchmarking
```bash
cd block_size_benchmarks
python3 run_benchmark.py \
  --config ../../../secrets/substrate/performance/performance.json \
  --from-time "2026-01-07T10:00:00Z" \
  --to-time "2026-01-07T10:10:00Z" \
  --node alice --node bob --node charlie
```

### Mempool Benchmarking
```bash
cd mempool_benchmarks
python3 run_mempool_benchmark.py \
  --config ../../../secrets/substrate/performance/performance.json \
  --from-time "2026-01-08T10:00:00Z" \
  --to-time "2026-01-08T10:10:00Z" \
  --window 1000
```

## Output

All benchmark results are saved in timestamped directories under `benchmarks/logs/` with:
- Downloaded node logs
- Extracted metrics and reports
- Statistical analysis
- Generated graphs (PNG)

## Notes

- Default node list includes 20 nodes: alice, bob, charlie, dave, eve, ferdie, george, henry, iris, jack, kate, leo, mike, nina, oliver, paul, quinn, rita, sam, tom
- Log files are automatically sorted by timestamp
- Existing log files are not re-downloaded
- All scripts support encrypted config files via sops
