# Time-Based Block Size Analysis

This document explains how to perform block size analysis based on specific time ranges using logs downloaded from Loki/Grafana.

## Overview

The enhanced `analyze_block_sizes.py` script now supports deriving block numbers from a time range by analyzing logs. This is useful when you want to analyze blocks that were produced during a specific time window (e.g., during a load test or performance benchmark).

## Workflow

The time-based analysis workflow consists of three steps:

1. **Download logs** for your time range using `download_logs.py`
2. **Extract block range** from the logs using `extract_block_range_from_logs.py`
3. **Analyze block sizes** using `analyze_block_sizes.py`

Steps 2 and 3 can be combined into a single command when using `analyze_block_sizes.py` with time range arguments.

## Quick Start

### Option 1: Integrated Workflow (Recommended)

Simplest approach - just specify the time range:

```bash
python3 analyze_block_sizes.py \
  --url ws://your-node:9944 \
  --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}'
```

The script will automatically:
- Check if logs already exist
- Download them if they don't exist
- Extract the block range
- Fetch and analyze block sizes

### Option 2: Manual Control

If you want more control over the process:

**Step 1: Download logs (if not already present)**

```bash
cd ../../  # Go to benchmarks directory
python3 download_logs.py \
  --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}' \
  --config ../secrets/substrate/performance/performance.json
cd block_size_benchmarks/block_size_analysis
```

**Step 2: Preview block range (optional)**

```bash
python3 extract_block_range_from_logs.py \
  --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}'
```

Note: `--log-dir` is optional; it will auto-discover logs.

**Step 3: Analyze block sizes**

```bash
python3 analyze_block_sizes.py \
  --url ws://your-node:9944 \
  --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}'
```

## Time Format

Time ranges can be specified in two formats:

### JSON Format

```bash
--time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}'
```

### Separate Parameters

```bash
--from-time "2026-01-26 16:20:00" --to-time "2026-01-26 16:40:00"
```

### Timezone Handling

- Times without timezone information (e.g., `2026-01-26 16:20:00`) are assumed to be in **EST (UTC-5)** and automatically converted to UTC
- This matches the behavior of `download_logs.py`
- You can also specify times in ISO 8601 format with explicit timezone:
  - `2026-01-26T16:20:00Z` (UTC)
  - `2026-01-26T11:20:00-05:00` (EST)

## Advanced Options

### Node Selection

By default, the script scans all log files. You can specify a specific node:

```bash
python3 analyze_block_sizes.py \
  --url ws://your-node:9944 \
  --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}' \
  --node alice
```

### Custom Log Directory

If logs are in a non-standard location:

```bash
python3 analyze_block_sizes.py \
  --url ws://your-node:9944 \
  --log-dir /path/to/custom/logs \
  --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}'
```

### Custom Config File

For log downloads, specify a custom config file:

```bash
python3 analyze_block_sizes.py \
  --url ws://your-node:9944 \
  --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}' \
  --config /path/to/config.json
```

## How It Works

### Block Extraction Logic

The `extract_block_range_from_logs.py` script:

1. Scans log files in the specified directory
2. For each log line:
   - Extracts the timestamp
   - Checks if it falls within the specified time range
   - If yes, extracts any block numbers from patterns like:
     - `Imported #42000`
     - `best: #42000`
     - `finalized #42000`
3. Returns the minimum and maximum block numbers found

### Log Format Requirements

The script expects logs to be in the format produced by `download_logs.py`, which includes:

- Timestamps in the format `YYYY-MM-DD HH:MM:SS.fff` or `[YYYY-MM-DD HH:MM:SS.fffZ]`
- Block numbers in Substrate log format (e.g., `Imported #1234`)

## Example Scenarios

### Analyzing a Load Test Period

```bash
# Single command - logs will be auto-downloaded if needed
python3 analyze_block_sizes.py \
  --url ws://your-node:9944 \
  --time-range '{"from":"2026-01-26 14:00:00","to":"2026-01-26 15:00:00"}'
```

### Using the Example Script

An example shell script is provided that demonstrates the complete workflow:

```bash
./example_time_based_analysis.sh
```

Edit the script to customize:
- Time range
- Node URL
- Config file path

## Output

The script generates the same outputs as regular block size analysis:

- `block_sizes.csv` - Raw block data
- `block_sizes_over_time.png` - Line chart
- `block_size_distribution.png` - Histogram
- `block_size_vs_extrinsics.png` - Scatter plot
- `block_size_analysis_dashboard.png` - Combined dashboard
- `block_size_report.txt` - Statistical summary

The output directory name will be auto-generated based on the block range, e.g., `block_size_analysis_41973_to_42009/`

## Troubleshooting

### No blocks found

If you get "No blocks found in the specified time range":

1. Check that the log directory contains log files
2. Verify the time range matches when blocks were actually produced
3. Try widening the time range
4. Check if logs contain block import messages (search for "Imported #")

### Incorrect block range

If the extracted block range seems wrong:

1. Verify timezone - remember that times without timezone are assumed to be EST
2. Check the log files manually to see what blocks were imported
3. Try specifying a specific node with `--node`

### Log format issues

If block extraction fails:

1. Ensure logs were downloaded using `download_logs.py`
2. Check that log files are in the expected format (plain text with timestamps)
3. Verify that logs contain Substrate-formatted block messages

## Integration with Existing Workflows

This feature integrates seamlessly with existing block size analysis tools:

- Use `--skip-fetch` to regenerate visualizations without re-fetching block data
- CSV output can be used with Jupyter notebooks for custom analysis
- Combine with other benchmark scripts for comprehensive performance analysis

## See Also

- `README_BLOCKSIZE.md` - General block size analysis documentation
- `../../download_logs.py` - Log downloading script
- `fetch_block_sizes.py` - Standalone block fetching script
- `plot_block_sizes.py` - Standalone visualization script
