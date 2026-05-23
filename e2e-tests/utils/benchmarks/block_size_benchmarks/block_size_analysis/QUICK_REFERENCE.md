# Quick Reference: Time-Based Block Size Analysis

## TL;DR

Analyze block sizes for a specific time period - just one command:

```bash
python3 analyze_block_sizes.py \
  --url ws://your-node:9944 \
  --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}'
```

The script automatically:
- Finds existing logs or downloads them if needed
- Extracts block numbers from the time range
- Fetches block data and generates visualizations

## Common Commands

### Use specific node
```bash
python3 analyze_block_sizes.py \
  --url ws://your-node:9944 \
  --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}' \
  --node alice
```

### Use separate time parameters
```bash
python3 analyze_block_sizes.py \
  --url ws://your-node:9944 \
  --from-time "2026-01-26 16:20:00" \
  --to-time "2026-01-26 16:40:00"
```

### Specify custom log directory
```bash
python3 analyze_block_sizes.py \
  --url ws://your-node:9944 \
  --log-dir /path/to/custom/logs \
  --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}'
```

### Extract block range only (preview)
```bash
python3 extract_block_range_from_logs.py \
  --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}'
```

### Get JSON output (for scripting)
```bash
python3 extract_block_range_from_logs.py \
  --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}' \
  --json
```

## Time Format Notes

- `"2026-01-26 16:20:00"` → Treated as **EST**, converted to UTC
- `"2026-01-26T16:20:00Z"` → Treated as **UTC**
- `"2026-01-26T11:20:00-05:00"` → Explicit **EST** timezone

## Files

- **analyze_block_sizes.py** - Main analysis script (updated with time-based support)
- **extract_block_range_from_logs.py** - Extract block range from logs (new)
- **example_time_based_analysis.sh** - Example workflow script (new)
- **TIME_BASED_ANALYSIS.md** - Detailed documentation (new)
- **README_BLOCKSIZE.md** - General documentation (updated)

## Example Output

```
Extracting block range from logs...
------------------------------------------------------------
  Converted EST time '2026-01-26T11:01:00' to UTC: 2026-01-26 16:01:00
  Converted EST time '2026-01-26T11:05:41' to UTC: 2026-01-26 16:05:41
Scanning 20 log file(s)...
  alice.txt: found 37 blocks
  bob.txt: found 36 blocks
  ...
Found block range: 41973 to 42009

STEP 1: Fetching block size data...
------------------------------------------------------------
Connecting to node at ws://127.0.0.1:9944...
Connected successfully!
Fetching blocks 41973 to 42009 (37 blocks)...
Progress: 100.0% - Block #42009: 12.34 KB (5 extrinsics)

STEP 2: Generating visualizations...
------------------------------------------------------------
...

ANALYSIS COMPLETE
All outputs saved to: block_size_analysis_41973_to_42009/
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "No blocks found" | Check time range, try widening it |
| Wrong timezone | Times without TZ are EST by default |
| Can't find logs | Verify `--log-dir` path is correct |
| Missing download_logs.py | Navigate to `../../` directory |

## See Also

- `TIME_BASED_ANALYSIS.md` - Full documentation
- `README_BLOCKSIZE.md` - General block size analysis docs
- `./example_time_based_analysis.sh` - Complete workflow example
