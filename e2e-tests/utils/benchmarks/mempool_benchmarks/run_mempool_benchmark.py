#!/usr/bin/env python3
"""
Mempool Benchmark Runner

This script automates the process of:
1. Downloading logs from Grafana/Loki for specified nodes (default: ferdie, charlie)
2. Extracting mempool metrics in-memory
3. Analyzing the metrics and generating reports/charts
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path
from datetime import datetime

# Ensure we can import local modules
script_dir = Path(__file__).parent.resolve()
sys.path.append(str(script_dir))

import extractor
import analyzer

def run_command(cmd, description, cwd=None):
    """Run a command and handle errors."""
    print(f"\n{'='*60}")
    print(f"{description}")
    print(f"{'='*60}")
    print(f"Command: {' '.join(cmd)}\n")

    try:
        result = subprocess.run(
            cmd,
            cwd=cwd,
            capture_output=True,
            text=True,
            check=True
        )
        print(result.stdout)
        if result.stderr:
            print("STDERR:", result.stderr)
        return True
    except subprocess.CalledProcessError as e:
        print(f"ERROR: {description} failed!")
        print(f"Exit code: {e.returncode}")
        print(f"STDOUT: {e.stdout}")
        print(f"STDERR: {e.stderr}")
        return False
    except Exception as e:
        print(f"ERROR: Unexpected error during {description}: {e}")
        return False


def main():
    parser = argparse.ArgumentParser(
        description="Run mempool benchmarking: download logs, extract, and analyze metrics.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Analyze default nodes (ferdie, charlie)
  python3 run_mempool_benchmark.py \\
    --config ../../secrets/substrate/performance/performance.json \\
    --from-time "2026-01-08T10:00:00Z" \\
    --to-time "2026-01-08T10:10:00Z"

  # Analyze specific node
  python3 run_mempool_benchmark.py \\
    --node charlie \\
    --config ../../secrets/substrate/performance/performance.json \\
    --from-time "2026-01-08T10:00:00Z" \\
    --to-time "2026-01-08T10:10:00Z"
        """
    )

    # Node selection
    parser.add_argument("--node", action='append',
                       help="Node to download logs from (can be used multiple times). Default: ['ferdie', 'charlie']")
    
    # Download options
    parser.add_argument("--config", 
                       default="../../../secrets/substrate/performance/performance.json",
                       help="Path to encrypted config file with Grafana credentials (default: ../../../secrets/substrate/performance/performance.json)")
    parser.add_argument("--url", help="Loki API URL (overrides config file)")
    parser.add_argument("--header", action='append', help="Custom header 'Key: Value' (can be used multiple times)")

    # Time range
    parser.add_argument("--time-range", help='Time range as JSON, e.g., \'{"from":"2026-01-20 10:34:25","to":"2026-01-20 11:34:25"}\'')
    parser.add_argument("--from-time", dest="start_time",
                       help="Start time in ISO 8601 format or YYYY-MM-DD HH:MM:SS (e.g., '2026-01-08T10:00:00Z')")
    parser.add_argument("--to-time", dest="end_time",
                       help="End time in ISO 8601 format or YYYY-MM-DD HH:MM:SS (e.g., '2026-01-08T10:10:00Z')")

    # Analysis options
    parser.add_argument("--window", type=int, default=None,
                       help="Time window in milliseconds for analysis (default: auto-calculated from time range)")
    parser.add_argument("--output-dir", default=".",
                       help="Output directory for all files (default: current directory)")

    # Skip steps
    parser.add_argument("--skip-download", action="store_true",
                       help="Skip log download (use existing logs)")

    args = parser.parse_args()
    
    # Default nodes if none specified
    if not args.node:
        args.node = ["ferdie", "charlie"]

    # Parse time range from JSON if provided
    if args.time_range:
        try:
            time_data = json.loads(args.time_range)
            args.start_time = time_data.get('from')
            args.end_time = time_data.get('to')
            if not args.start_time or not args.end_time:
                print("Error: time-range JSON must contain 'from' and 'to' fields")
                sys.exit(1)
        except json.JSONDecodeError as e:
            print(f"Error: Invalid JSON in --time-range: {e}")
            sys.exit(1)
    
    # Validate that we have time range (unless skipping download)
    if not args.skip_download and (not args.start_time or not args.end_time):
        print("Error: Either --time-range or both --from-time and --to-time must be provided")
        sys.exit(1)

    # Resolve output directory
    output_dir = Path(args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    # Calculate window if not provided
    if args.window is None:
        # Parse time range to calculate total duration
        start_time_normalized = args.start_time.replace(' ', 'T').replace('Z', '+00:00')
        end_time_normalized = args.end_time.replace(' ', 'T').replace('Z', '+00:00')
        start = datetime.fromisoformat(start_time_normalized)
        end = datetime.fromisoformat(end_time_normalized)
        duration_seconds = (end - start).total_seconds()
        # Use 1 second window as default
        args.window = 1000
        print(f"\nAuto-calculated window: {args.window}ms for {duration_seconds:.0f}s duration")

    print(f"\n{'#'*60}")
    print(f"# MEMPOOL BENCHMARK RUNNER")
    print(f"{'#'*60}")
    print(f"Nodes: {', '.join(args.node)}")
    print(f"Time range: {args.start_time} to {args.end_time}")
    print(f"Analysis window: {args.window}ms")
    print(f"Output directory: {output_dir}")
    print(f"Script directory: {script_dir}")

    # Step 1: Download logs
    log_dir = None
    if not args.skip_download:
        # Construct expected directory name
        try:
            start_time_normalized = args.start_time.replace(' ', 'T').replace('Z', '+00:00')
            end_time_normalized = args.end_time.replace(' ', 'T').replace('Z', '+00:00')
            start_dt = datetime.fromisoformat(start_time_normalized)
            end_dt = datetime.fromisoformat(end_time_normalized)
            start_str = start_dt.strftime("%Y-%m-%d_%H-%M-%S")
            end_str = end_dt.strftime("%Y-%m-%d_%H-%M-%S")
            date_range_folder = f"from_{start_str}_to_{end_str}"
        except Exception:
            start_str = args.start_time.replace(':', '-').replace('T', '_').replace(' ', '_')
            end_str = args.end_time.replace(':', '-').replace('T', '_').replace(' ', '_')
            date_range_folder = f"from_{start_str}_to_{end_str}"
        
        if args.output_dir != ".":
            base_path = output_dir
        else:
            base_path = script_dir.parent / "logs"
        
        log_dir = base_path / date_range_folder

        # Download for each node
        for node in args.node:
            download_cmd = [
                sys.executable,
                str(script_dir.parent / "download_logs.py"),
                "--node", node,
                "--from-time", args.start_time,
                "--to-time", args.end_time
            ]

            if args.output_dir != ".":
                download_cmd.extend(["--output-dir", str(output_dir)])

            if args.config:
                download_cmd.extend(["--config", args.config])
            if args.url:
                download_cmd.extend(["--url", args.url])
            if args.header:
                for header in args.header:
                    download_cmd.extend(["--header", header])

            if not run_command(download_cmd, f"Step 1: Downloading {node}'s logs"):
                # We prioritize success, but if one fails we might continue? 
                # Better to fail if data is missing.
                sys.exit(1)
                
        print(f"\nLogs located in: {log_dir}")
        
    else:
        print("\nStep 1: SKIPPED (using existing logs)")
        # If skipping download, assume log dir is output dir or we need to find it?
        # The user provided output_dir is likely where they expect results.
        # But if they skip download, we need to know where the logs ARE.
        # Assuming they are in output_dir if specified, or current dir.
        log_dir = output_dir
    
    if not log_dir.exists():
        print(f"ERROR: Log directory {log_dir} does not exist!")
        sys.exit(1)

    # Change to log directory context for extracting (extractor usually looks in CWD or we pass paths)
    import os
    original_cwd = os.getcwd()
    os.chdir(log_dir)
    
    try:
        # Step 2: Extract metrics (In Memory)
        print(f"\nStep 2: Extracting mempool metrics for {', '.join(args.node)}...")
        events = extractor.parse_logs(args.node)
        print(f"Found {len(events)} mempool events")

        if not events:
            print("No events found! Exiting.")
            sys.exit(1)

        # Step 3: Analyze metrics (In Memory)
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        analysis_file = f"mempool_analysis_{timestamp}.txt"
        
        print(f"\nStep 3: Analyzing metrics (Window: {args.window}ms)...")
        
        # Convert extractor.MempoolEvent to analyzer.MempoolPoint
        points = []
        for e in events:
            points.append(analyzer.MempoolPoint(
                timestamp=e.timestamp,
                node=e.node,
                ready=e.ready,
                future=e.future,
                mempool_len=e.mempool_len,
                submitted_count=e.submitted_count,
                validated_count=e.validated_count,
                revalidated=e.revalidated,
                pruned_count=e.pruned_count,
                reverified_txs=e.reverified_txs
            ))
            
        analyzer.analyze_data(points, args.window, analysis_file)
        
        print(f"\n{'='*60}")
        print("SUCCESS! Mempool benchmark complete.")
        print(f"{'='*60}")
        print(f"Output directory: {log_dir}")
        print(f"Files generated:")
        print(f"  - {analysis_file}: Analysis and insights")
        print(f"  - {analysis_file.replace('.txt', '_timeseries.csv')}: Time-series data")
        print(f"  - {analysis_file.replace('.txt', '_mempool.png')}: Charts")

    finally:
        os.chdir(original_cwd)

if __name__ == "__main__":
    main()
