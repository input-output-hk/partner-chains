# Updated Mempool Benchmark Script

## Changes Made

The `run_mempool_benchmark.py` script has been updated to support analyzing **any node**, not just Ferdie.

### New Parameter

- `--node <node_name>` - Specify which node to analyze (default: ferdie)
  - Examples: `ferdie`, `charlie`, `alice`, `bob`, etc.

## Usage Examples

### Analyze Charlie (Validator Node)

```bash
cd /Users/larry/Project/iohk/partner-chains/e2e-tests/utils/mempool_benchmarks

python3 run_mempool_benchmark.py \
  --node charlie \
  --config ../../secrets/substrate/performance/performance.json \
  --from-time "2026-01-13T14:54:00Z" \
  --to-time "2026-01-13T15:04:00Z"
```

### Analyze Ferdie (Relay Node) - Default

```bash
python3 run_mempool_benchmark.py \
  --config ../../secrets/substrate/performance/performance.json \
  --from-time "2026-01-13T14:54:00Z" \
  --to-time "2026-01-13T15:04:00Z"
```

## Comparing Ferdie vs Charlie

To compare relay node (Ferdie) vs validator node (Charlie):

1. **Run benchmark for Charlie:**
   ```bash
   python3 run_mempool_benchmark.py \
     --node charlie \
     --config ../../secrets/substrate/performance/performance.json \
     --from-time "2026-01-13T14:54:00Z" \
     --to-time "2026-01-13T15:04:00Z" \
     --output-dir benchmark_results/charlie_comparison
   ```

2. **Run benchmark for Ferdie:**
   ```bash
   python3 run_mempool_benchmark.py \
     --node ferdie \
     --config ../../secrets/substrate/performance/performance.json \
     --from-time "2026-01-13T14:54:00Z" \
     --to-time "2026-01-13T15:04:00Z" \
     --output-dir benchmark_results/ferdie_comparison
   ```

3. **Compare the analysis reports:**
   - Look at `Total Validated` - Charlie (validator) should show non-zero values
   - Compare `Total Revalidated`, `Total Pruned`, and `Total Reverified`
   - Compare mempool utilization patterns

## Expected Differences

### Charlie (Validator)
- ✅ Should show **validated** transactions (receives direct submissions)
- ✅ Higher mempool utilization
- ✅ More transaction events

### Ferdie (Relay)
- ❌ Zero validated transactions (doesn't receive direct submissions)
- Lower mempool utilization
- Primarily propagates blocks and revalidates

## Output Files

Each run creates:
- `{node}.txt` - Raw logs
- `mempool_report.txt` - Formatted report
- `mempool_events.csv` - Raw event data for graphing
- `mempool_analysis_*.txt` - Analysis with insights
- `mempool_analysis_*_timeseries.csv` - Time-series data for graphing
