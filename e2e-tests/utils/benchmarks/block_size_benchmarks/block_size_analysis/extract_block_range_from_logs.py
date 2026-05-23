#!/usr/bin/env python3
"""
Extract block number range from logs based on a time range.
This script reads logs downloaded by download_logs.py and finds the block numbers
that correspond to the given time range.
"""

import argparse
import re
import json
import sys
from pathlib import Path
from datetime import datetime, timezone


def parse_block_number_from_log_line(line):
    """Extract block number from a log line if present."""
    # Look for patterns like "Imported #1234" or "best: #1234" which are common in Substrate logs
    # Also look for db-sync-node patterns like "block 3978787"
    patterns = [
        r'Imported #(\d+)',
        r'best:\s*#(\d+)',
        r'finalized[:\s]*#(\d+)',
        r'prepared block for proposing #(\d+)',
        r'Highest known block at #(\d+)',
    ]
    
    for pattern in patterns:
        match = re.search(pattern, line, re.IGNORECASE)
        if match:
            return int(match.group(1))
    return None


def parse_timestamp_from_log_line(line):
    """Extract timestamp from a log line if present."""
    # Look for ISO 8601 timestamp patterns
    # Example: 2026-01-26T16:20:00.123Z or 2026-01-26 16:20:00 or [2026-01-26 16:20:00.123 UTC]
    patterns = [
        r'\[(\d{4}-\d{2}-\d{2}[T\s]\d{2}:\d{2}:\d{2}(?:\.\d+)?)(?: UTC)?\]',  # [timestamp] or [timestamp UTC]
        r'^(\d{4}-\d{2}-\d{2}[T\s]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?)',    # start of line
    ]
    
    for pattern in patterns:
        match = re.search(pattern, line)
        if match:
            timestamp_str = match.group(1)
            try:
                # Parse the timestamp
                if 'T' in timestamp_str or 'Z' in timestamp_str:
                    if timestamp_str.endswith('Z'):
                        timestamp_str = timestamp_str[:-1] + '+00:00'
                    # Handle timestamps with just 'Z' but no 'T'
                    if 'Z' in timestamp_str and 'T' not in timestamp_str:
                        timestamp_str = timestamp_str.replace(' ', 'T')
                    dt = datetime.fromisoformat(timestamp_str)
                else:
                    # Space-separated format, try parsing with microseconds first
                    try:
                        dt = datetime.strptime(timestamp_str, '%Y-%m-%d %H:%M:%S.%f')
                    except ValueError:
                        dt = datetime.strptime(timestamp_str, '%Y-%m-%d %H:%M:%S')
                    dt = dt.replace(tzinfo=timezone.utc)
                return dt
            except ValueError:
                continue
    return None


def extract_blocks_from_log_file(log_file, start_time, end_time, min_block_threshold=0):
    """Extract block numbers from a log file within the given time range.
    
    Args:
        log_file: Path to the log file
        start_time: Start of time range
        end_time: End of time range
        min_block_threshold: Minimum block number to consider (filters out substrate chain blocks)
    """
    blocks = set()
    
    try:
        with open(log_file, 'r', encoding='utf-8') as f:
            for line in f:
                # Parse timestamp from line
                timestamp = parse_timestamp_from_log_line(line)
                if not timestamp:
                    continue
                
                # Check if timestamp is within range
                if start_time <= timestamp <= end_time:
                    block_num = parse_block_number_from_log_line(line)
                    # Filter out low block numbers (substrate chain) and focus on high blocks (Cardano chain)
                    if block_num is not None and block_num >= min_block_threshold:
                        blocks.add(block_num)
    except Exception as e:
        print(f"Warning: Error reading {log_file}: {e}", file=sys.stderr)
    
    return blocks


def find_block_range_from_logs(log_dir, start_time, end_time, node_name=None, min_block_threshold=0):
    """Find the block range from logs in the given directory.
    
    Args:
        log_dir: Directory containing log files
        start_time: Start of time range
        end_time: End of time range
        node_name: Optional specific node to analyze
        min_block_threshold: Minimum block number to consider (default: 100000)
    """
    log_dir = Path(log_dir)
    
    if not log_dir.exists():
        print(f"Error: Log directory not found: {log_dir}", file=sys.stderr)
        raise FileNotFoundError(f"Log directory not found: {log_dir}")
    
    # Find log files
    if node_name:
        if not log_files[0].exists():
            print(f"Error: Log file not found: {log_files[0]}", file=sys.stderr)
            raise FileNotFoundError(f"Log file not found: {log_files[0]}")
    else:
        log_files = list(log_dir.glob("*.txt"))
        # Exclude special files
        log_files = [f for f in log_files if not f.name.startswith('.') and f.name != 'log_run_details.json']
    
    if not log_files:
        print(f"Error: No log files found in {log_dir}", file=sys.stderr)
        raise FileNotFoundError(f"No log files found in {log_dir}")
    
    print(f"Scanning {len(log_files)} log file(s)... (min block: {min_block_threshold})", file=sys.stderr)
    
    all_blocks = set()
    for log_file in log_files:
        blocks = extract_blocks_from_log_file(log_file, start_time, end_time, min_block_threshold)
        all_blocks.update(blocks)
        if blocks:
            print(f"  {log_file.name}: found {len(blocks)} blocks", file=sys.stderr)
    
    if not all_blocks:
        print("Error: No blocks found in the specified time range", file=sys.stderr)
        raise ValueError("No blocks found in the specified time range")
    
    # Heuristic: find largest contiguous range (handles chain restarts)
    sorted_blocks = sorted(all_blocks)
    
    # Filter by min_block_threshold first
    sorted_blocks = [b for b in sorted_blocks if b >= min_block_threshold]
    
    if not sorted_blocks:
        print("Error: No blocks found above minimum threshold", file=sys.stderr)
        raise ValueError("No blocks found above minimum threshold")
    
    # Find gaps (chain restarts create large gaps)
    # Group blocks into contiguous ranges with max gap of 100 blocks
    ranges = []
    current_range = [sorted_blocks[0]]
    
    for block in sorted_blocks[1:]:
        if block - current_range[-1] <= 100:  # Allow small gaps
            current_range.append(block)
        else:
            # Gap too large, start new range
            ranges.append(current_range)
            current_range = [block]
    ranges.append(current_range)
    
    # Select the best range: prefer larger ranges, but if similar size, prefer higher block numbers
    # (higher numbers = established chain, lower numbers = fresh restart)
    def range_score(r):
        # Primary: number of blocks, Secondary: max block number
        return (len(r), max(r))
    
    best_range = max(ranges, key=range_score)
    min_block = min(best_range)
    max_block = max(best_range)
    
    if len(ranges) > 1:
        print(f"\nNote: Found {len(ranges)} separate block ranges (likely chain restarts).", file=sys.stderr)
        for i, r in enumerate(ranges):
            print(f"  Range {i+1}: #{min(r)} to #{max(r)} ({len(r)} blocks)", file=sys.stderr)
        print(f"Selected range: #{min_block} to #{max_block} ({len(best_range)} blocks)", file=sys.stderr)
    
    return min_block, max_block


def parse_time(time_str):
    """Parse time string to datetime object.
    
    Times without timezone info are assumed to be in EST and converted to UTC,
    matching the behavior of download_logs.py.
    """
    try:
        # Handle Z for UTC
        if time_str.endswith("Z"):
            time_str = time_str[:-1] + "+00:00"
        
        # Try to parse space-separated format (YYYY-MM-DD HH:MM:SS)
        if ' ' in time_str and 'T' not in time_str:
            time_str = time_str.replace(' ', 'T')
        
        # Parse ISO format
        dt = datetime.fromisoformat(time_str)
        
        # If no timezone info, assume EST (UTC-5) and convert to UTC
        # This matches the behavior of download_logs.py
        if dt.tzinfo is None:
            from datetime import timedelta
            # Assume time is in EST (UTC-5)
            dt = dt.replace(tzinfo=timezone(timedelta(hours=-5)))
            # Convert to UTC
            dt = dt.astimezone(timezone.utc)
            print(f"  Converted EST time '{time_str}' to UTC: {dt.strftime('%Y-%m-%d %H:%M:%S')}", file=sys.stderr)
        
        return dt
    except ValueError as e:
        print(f"Error parsing time '{time_str}': {e}", file=sys.stderr)
        print("Please use ISO 8601 format (e.g., '2026-01-26T16:20:00Z') or 'YYYY-MM-DD HH:MM:SS' (assumed EST)", file=sys.stderr)
        raise ValueError(f"Invalid time format: {time_str}")


def main():
    parser = argparse.ArgumentParser(
        description="Extract block number range from logs based on time range",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Using time range JSON
  %(prog)s --log-dir ./logs/from_2026-01-26_16-20-00_to_2026-01-26_16-40-00 \\
    --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}'
  
  # Using separate from/to times
  %(prog)s --log-dir ./logs/my_logs \\
    --from-time "2026-01-26 16:20:00" \\
    --to-time "2026-01-26 16:40:00"
  
  # Specify a specific node
  %(prog)s --log-dir ./logs/my_logs \\
    --from-time "2026-01-26 16:20:00" \\
    --to-time "2026-01-26 16:40:00" \\
    --node alice
        """
    )
    
    parser.add_argument("--log-dir",
                       help="Directory containing log files (optional - will auto-discover if not specified)")
    
    parser.add_argument("--time-range",
                       help='Time range as JSON, e.g., \'{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}\'')
    
    parser.add_argument("--from-time",
                       help="Start time (ISO 8601 or YYYY-MM-DD HH:MM:SS)")
    
    parser.add_argument("--to-time",
                       help="End time (ISO 8601 or YYYY-MM-DD HH:MM:SS)")
    
    parser.add_argument("--node",
                       help="Specific node to analyze (default: analyze all nodes)")
    
    parser.add_argument("--min-block",
                       type=int,
                       default=0,
                       help="Minimum block number to consider (default: 0)")
    
    parser.add_argument("--json",
                       action="store_true",
                       help="Output result as JSON")
    
    args = parser.parse_args()
    
    # Parse time range from JSON if provided
    if args.time_range:
        try:
            time_data = json.loads(args.time_range)
            start_time_str = time_data.get('from')
            end_time_str = time_data.get('to')
            if not start_time_str or not end_time_str:
                print("Error: time-range JSON must contain 'from' and 'to' fields", file=sys.stderr)
                sys.exit(1)
        except json.JSONDecodeError as e:
            print(f"Error: Invalid JSON in --time-range: {e}", file=sys.stderr)
            sys.exit(1)
    else:
        start_time_str = args.from_time
        end_time_str = args.to_time
    
    # Validate that we have time range
    if not start_time_str or not end_time_str:
        print("Error: Either --time-range or both --from-time and --to-time must be provided", file=sys.stderr)
        sys.exit(1)
    
    # Parse times
    start_time = parse_time(start_time_str)
    end_time = parse_time(end_time_str)
    
    # Auto-discover log directory if not specified
    if not args.log_dir:
        try:
            def parse_dt(time_str):
                if 'T' in time_str or 'Z' in time_str:
                    time_str = time_str.replace('Z', '+00:00')
                    return datetime.fromisoformat(time_str)
                else:
                    return datetime.strptime(time_str, '%Y-%m-%d %H:%M:%S')
            
            start_dt = parse_dt(start_time_str)
            end_dt = parse_dt(end_time_str)
            start_str = start_dt.strftime("%Y-%m-%d_%H-%M-%S")
            end_str = end_dt.strftime("%Y-%m-%d_%H-%M-%S")
            
            script_dir = Path(__file__).parent
            expected_log_dir = script_dir / ".." / ".." / "logs" / f"from_{start_str}_to_{end_str}"
            expected_log_dir = expected_log_dir.resolve()
            
            if expected_log_dir.exists() and list(expected_log_dir.glob("*.txt")):
                args.log_dir = str(expected_log_dir)
                print(f"Auto-discovered logs: {args.log_dir}", file=sys.stderr)
            else:
                print(f"Error: Could not find logs at {expected_log_dir}", file=sys.stderr)
                print(f"Please download logs first using download_logs.py or specify --log-dir", file=sys.stderr)
                sys.exit(1)
        except Exception as e:
            print(f"Error: Could not auto-discover log directory: {e}", file=sys.stderr)
            print(f"Please specify --log-dir explicitly", file=sys.stderr)
            sys.exit(1)
    
    # Find block range
    min_block, max_block = find_block_range_from_logs(
        args.log_dir,
        start_time,
        end_time,
        args.node,
        args.min_block
    )
    
    # Output results
    if args.json:
        result = {
            "start_block": min_block,
            "end_block": max_block,
            "total_blocks": max_block - min_block + 1
        }
        print(json.dumps(result))
    else:
        print(f"\nBlock range found:", file=sys.stderr)
        print(f"  Start block: {min_block}", file=sys.stderr)
        print(f"  End block: {max_block}", file=sys.stderr)
        print(f"  Total blocks: {max_block - min_block + 1}", file=sys.stderr)
        print(f"\nYou can use these with analyze_block_sizes.py:", file=sys.stderr)
        print(f"  --start-block {min_block} --end-block {max_block}", file=sys.stderr)


if __name__ == "__main__":
    main()
