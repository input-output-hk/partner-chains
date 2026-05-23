#!/usr/bin/env python3
import re
import sys
import subprocess
import argparse
from pathlib import Path
import json
from datetime import datetime

def count_block_transactions(file_path):
    """
    Parses a Midnight/Substrate log file to count distinct validated transactions.
    """
    # A set is used to automatically handle deduplication
    distinct_hashes = set()

    # Regex to find the specific confirmation message and capture the hex hash
    # Example match: Validated Midnight transaction "618804..."
    tx_pattern = re.compile(r'Validated Midnight transaction "([a-fA-F0-9]+)"')

    try:
        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
            for line in f:
                # search() finds the pattern anywhere in the line
                match = tx_pattern.search(line)
                if match:
                    # distinct_hashes stores the unique ID found in group 1
                    distinct_hashes.add(match.group(1))
        
        return distinct_hashes

    except FileNotFoundError:
        print(f"❌ Error: The file '{file_path}' was not found.")
        return set()
    except Exception as e:
        print(f"❌ An error occurred: {e}")
        return set()

def download_logs(from_time, to_time, nodes, config, header, url):
    """
    Calls the download_logs.py script to download logs for specified nodes.
    Returns the path to the directory containing the downloaded logs.
    """
    download_script = Path(__file__).parent.parent / "download_logs.py"
    
    if not download_script.exists():
        print(f"❌ Error: download_logs.py not found at {download_script}")
        sys.exit(1)
    
    cmd = [
        sys.executable,
        str(download_script),
        "--from-time", from_time,
        "--to-time", to_time
    ]
    
    # Add config if provided
    if config:
        cmd.extend(["--config", config])
    
    # Add URL if provided
    if url:
        cmd.extend(["--url", url])
    
    # Add nodes
    if nodes:
        for node in nodes:
            cmd.extend(["--node", node])
    
    # Add headers if provided
    if header:
        for h in header:
            cmd.extend(["--header", h])
    
    print(f"Downloading logs...")
    print(f"Running: {' '.join(cmd)}\n")
    
    try:
        result = subprocess.run(cmd, check=True, capture_output=True, text=True)
        print(result.stdout)
        
        # Construct expected directory name based on date range
        # This matches the logic in download_logs.py
        try:
            start_dt = datetime.fromisoformat(from_time.replace('Z', '+00:00'))
            end_dt = datetime.fromisoformat(to_time.replace('Z', '+00:00'))
            start_str = start_dt.strftime("%Y-%m-%d_%H-%M-%S")
            end_str = end_dt.strftime("%Y-%m-%d_%H-%M-%S")
            date_range_folder = f"from_{start_str}_to_{end_str}"
        except Exception:
            # Fallback to simple names
            start_str = from_time.replace(':', '-').replace('T', '_')
            end_str = to_time.replace(':', '-').replace('T', '_')
            date_range_folder = f"from_{start_str}_to_{end_str}"
        
        # download_logs.py uses its own script_dir/logs
        base_path = download_script.parent / "logs"
        
        # Expected log directory
        log_dir = base_path / date_range_folder
        
        if not log_dir.exists():
            print(f"❌ Error: Expected log directory does not exist: {log_dir}")
            sys.exit(1)
        
        print(f"\nUsing logs in: {log_dir}\n")
        return log_dir
        
    except subprocess.CalledProcessError as e:
        print(f"❌ Error downloading logs: {e}")
        if e.stdout:
            print(f"STDOUT:\n{e.stdout}")
        if e.stderr:
            print(f"STDERR:\n{e.stderr}")
        sys.exit(1)

def process_all_logs(log_dir):
    """
    Process all log files in the given directory and count transactions.
    """
    log_dir = Path(log_dir)
    
    if not log_dir.exists():
        print(f"❌ Error: Log directory '{log_dir}' does not exist.")
        sys.exit(1)
    
    # Find all .txt log files
    log_files = list(log_dir.glob("*.txt"))
    
    if not log_files:
        print(f"❌ Error: No log files found in '{log_dir}'")
        sys.exit(1)
    
    print(f"\nProcessing {len(log_files)} log files...\n")
    
    # Collect all unique transactions across all nodes
    all_txs = set()
    node_results = {}
    
    for log_file in sorted(log_files):
        node_name = log_file.stem
        node_txs = count_block_transactions(log_file)
        node_results[node_name] = len(node_txs)
        all_txs.update(node_txs)
        print(f"{node_name}: {len(node_txs)} distinct transactions")
    
    return all_txs, node_results

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Download logs and count distinct validated transactions across all nodes."
    )
    
    # Mode selection
    parser.add_argument(
        "--download", 
        action="store_true",
        help="Download logs before processing"
    )
    
    # Download parameters
    parser.add_argument("--time-range", help='Time range as JSON, e.g., \'{"from":"2026-01-20 10:34:25","to":"2026-01-20 11:34:25"}\'')
    parser.add_argument("--from-time", help="Start time (ISO 8601 or YYYY-MM-DD HH:MM:SS, e.g., 2026-01-14T05:37:00Z)")
    parser.add_argument("--to-time", help="End time (ISO 8601 or YYYY-MM-DD HH:MM:SS)")
    parser.add_argument("--node", action="append", help="Node to download (can be used multiple times)")
    parser.add_argument("--header", action="append", help="Authorization header (e.g., 'Authorization: Bearer TOKEN'). Can be used multiple times.")
    parser.add_argument("--url", help="Grafana/Loki URL")
    parser.add_argument("--config", 
                       default="../../../secrets/substrate/performance/performance.json",
                       help="Path to encrypted config file (default: ../../../secrets/substrate/performance/performance.json)")
    
    # Processing parameters
    parser.add_argument(
        "--log-dir",
        help="Path to directory containing log files (alternative to --download)"
    )
    parser.add_argument(
        "log_file",
        nargs="?",
        help="Single log file to process (legacy mode)"
    )
    
    args = parser.parse_args()
    
    # Parse time range from JSON if provided
    if args.time_range:
        try:
            time_data = json.loads(args.time_range)
            args.from_time = time_data.get('from')
            args.to_time = time_data.get('to')
            if not args.from_time or not args.to_time:
                print("Error: time-range JSON must contain 'from' and 'to' fields")
                sys.exit(1)
        except json.JSONDecodeError as e:
            print(f"Error: Invalid JSON in --time-range: {e}")
            sys.exit(1)
    
    # Legacy mode: single file processing
    if args.log_file:
        print(f"Scanning '{args.log_file}' for transactions...")
        unique_txs = count_block_transactions(args.log_file)
        count = len(unique_txs)
        print("-" * 30)
        print(f"Total Distinct Txs: {count}")
        print("-" * 30)
        if count > 0:
            print("First 5 hashes found:")
            for tx in list(unique_txs)[:5]:
                print(f" - {tx}")
        sys.exit(0)
    
    # Download mode
    if args.download:
        if not args.from_time or not args.to_time:
            print("❌ Error: --from-time and --to-time are required when using --download")
            sys.exit(1)
        
        # Use default nodes if none specified
        nodes = args.node if args.node else None
        
        log_dir = download_logs(
            args.from_time,
            args.to_time,
            nodes,
            args.config,
            args.header,
            args.url
        )
        
        if not log_dir:
            print("❌ Error: Could not determine log directory after download")
            sys.exit(1)
    
    # Process logs from specified directory
    elif args.log_dir:
        log_dir = Path(args.log_dir)
    else:
        print("❌ Error: Must specify either --download, --log-dir, or a log file")
        parser.print_help()
        sys.exit(1)
    
    # Process all logs in the directory
    all_txs, node_results = process_all_logs(log_dir)
    
    print("\n" + "=" * 50)
    print(f"TOTAL DISTINCT TRANSACTIONS ACROSS ALL NODES: {len(all_txs)}")
    print("=" * 50)
    
    # Optional: Print the first few hashes to verify
    if len(all_txs) > 0:
        print("\nFirst 5 transaction hashes found:")
        for tx in list(all_txs)[:5]:
            print(f" - {tx}")
