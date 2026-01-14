#!/usr/bin/env python3
"""
Mempool Benchmark Runner

This script automates the process of:
1. Downloading Ferdie's logs from Grafana/Loki
2. Extracting mempool metrics
3. Analyzing the metrics with configurable time windows
"""

import argparse
import subprocess
import sys
from pathlib import Path
from datetime import datetime


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
  # Analyze Ferdie (relay node, default)
  python3 run_mempool_benchmark.py \\
    --config ../../secrets/substrate/performance/performance.json \\
    --from-time "2026-01-08T10:00:00Z" \\
    --to-time "2026-01-08T10:10:00Z"

  # Analyze Charlie (validator node)
  python3 run_mempool_benchmark.py \\
    --node charlie \\
    --config ../../secrets/substrate/performance/performance.json \\
    --from-time "2026-01-08T10:00:00Z" \\
    --to-time "2026-01-08T10:10:00Z"

  # Custom time window for analysis (500ms instead of default 1s)
  python3 run_mempool_benchmark.py \\
    --node charlie \\
    --config config.json \\
    --from-time "2026-01-08T10:00:00Z" \\
    --to-time "2026-01-08T10:10:00Z" \\
    --window 500
        """
    )

    # Node selection
    parser.add_argument("--node", default="ferdie", 
                       help="Node to download logs from (default: ferdie)")
    
    # Download options
    parser.add_argument("--config", help="Path to encrypted config file with Grafana credentials")
    parser.add_argument("--url", help="Loki API URL (overrides config file)")
    parser.add_argument("--header", action='append', help="Custom header 'Key: Value' (can be used multiple times)")

    # Time range (required)
    parser.add_argument("--from-time", required=True, dest="start_time",
                       help="Start time in ISO 8601 format (e.g., '2026-01-08T10:00:00Z')")
    parser.add_argument("--to-time", required=True, dest="end_time",
                       help="End time in ISO 8601 format (e.g., '2026-01-08T10:10:00Z')")

    # Analysis options
    parser.add_argument("--window", type=int, default=1000,
                       help="Time window in milliseconds for analysis (default: 1000ms)")
    parser.add_argument("--output-dir", default=".",
                       help="Output directory for all files (default: current directory)")

    # Skip steps
    parser.add_argument("--skip-download", action="store_true",
                       help="Skip log download (use existing ferdie.txt)")
    parser.add_argument("--skip-extract", action="store_true",
                       help="Skip extraction (use existing mempool_report.txt)")

    args = parser.parse_args()

    # Resolve output directory
    output_dir = Path(args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    script_dir = Path(__file__).parent.resolve()

    print(f"\n{'#'*60}")
    print(f"# MEMPOOL BENCHMARK RUNNER")
    print(f"{'#'*60}")
    print(f"Node: {args.node}")
    print(f"Time range: {args.start_time} to {args.end_time}")
    print(f"Analysis window: {args.window}ms")
    print(f"Output directory: {output_dir}")
    print(f"Script directory: {script_dir}")

    # Step 1: Download logs
    if not args.skip_download:
        download_cmd = [
            "python3",
            str(script_dir / "../download_logs.py"),
            "--node", args.node,
            "--from-time", args.start_time,
            "--to-time", args.end_time,
            "--output-dir", str(output_dir)
        ]

        if args.config:
            download_cmd.extend(["--config", args.config])
        if args.url:
            download_cmd.extend(["--url", args.url])
        if args.header:
            for header in args.header:
                download_cmd.extend(["--header", header])

        if not run_command(download_cmd, f"Step 1: Downloading {args.node}'s logs"):
            sys.exit(1)

        # Find the most recent timestamped directory
        subdirs = [d for d in output_dir.iterdir() if d.is_dir()]
        if not subdirs:
            print("ERROR: No timestamped directories found after download!")
            sys.exit(1)
        
        # Get the most recent directory
        log_dir = max(subdirs, key=lambda d: d.stat().st_mtime)
        print(f"\nUsing log directory: {log_dir}")
        
        # Check that node log file exists in the directory
        node_file = log_dir / f"{args.node}.txt"
        if not node_file.exists():
            print(f"ERROR: {node_file} not found in downloaded directory!")
            sys.exit(1)
    else:
        print("\nStep 1: SKIPPED (using existing logs)")
        log_dir = output_dir
        node_file = log_dir / f"{args.node}.txt"
        if not node_file.exists():
            print(f"ERROR: {node_file} not found!")
            sys.exit(1)

    # Step 2: Extract metrics
    if not args.skip_extract:
        extract_cmd = [
            "python3",
            str(script_dir / "extractor.py"),
            args.node
        ]

        if not run_command(extract_cmd, "Step 2: Extracting mempool metrics", cwd=log_dir):
            sys.exit(1)

        # Check if report was created
        report_file = log_dir / "mempool_report.txt"
        if not report_file.exists():
            print("ERROR: mempool_report.txt was not created!")
            sys.exit(1)
        print(f"\n✓ Created: {report_file}")
    else:
        print("\nStep 2: SKIPPED (using existing mempool_report.txt)")
        report_file = log_dir / "mempool_report.txt"
        if not report_file.exists():
            print(f"ERROR: {report_file} not found!")
            sys.exit(1)

    # Step 3: Analyze metrics
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    analysis_file = log_dir / f"mempool_analysis_{timestamp}.txt"

    analyze_cmd = [
        "python3",
        str(script_dir / "analyzer.py"),
        "mempool_report.txt",
        str(analysis_file.name),
        str(args.window)
    ]

    if not run_command(analyze_cmd, "Step 3: Analyzing metrics", cwd=log_dir):
        sys.exit(1)

    # Print summary
    print(f"\n{'='*60}")
    print("SUCCESS! Mempool benchmark complete.")
    print(f"{'='*60}")
    print(f"\nOutput files in: {log_dir}")
    print(f"  - {args.node}.txt: Downloaded logs")
    print(f"  - mempool_report.txt: Extracted time-series data")
    print(f"  - mempool_events.csv: Raw event data (CSV)")
    print(f"  - {analysis_file.name}: Analysis and statistics")
    print(f"  - {analysis_file.name.rsplit('.', 1)[0]}_timeseries.csv: Time-series data (CSV)")
    print(f"\nTo view analysis:")
    print(f"  cat {analysis_file}")
    print()


if __name__ == "__main__":
    main()
