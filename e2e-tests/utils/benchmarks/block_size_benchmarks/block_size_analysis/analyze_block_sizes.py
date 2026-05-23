#!/usr/bin/env python3
"""
All-in-one script to fetch block sizes from a Substrate node and generate visualizations.
"""

import argparse
import subprocess
import sys
import json
import os
from pathlib import Path
from datetime import datetime

# Add current directory to path to import sibling modules
script_dir = Path(__file__).parent.absolute()
sys.path.append(str(script_dir))

import fetch_block_sizes
import plot_block_sizes

def main():
    parser = argparse.ArgumentParser(
        description="Fetch and visualize block sizes from a Substrate node",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Analyze latest 100 blocks from local node
  %(prog)s --url ws://127.0.0.1:9944 --latest-n 100
  
  # Analyze specific block range
  %(prog)s --url ws://127.0.0.1:9944 --start-block 1000 --end-block 2000
  
  # Analyze blocks from a time range (auto-discovers or downloads logs)
  %(prog)s --url ws://127.0.0.1:9944 --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}'
  
  # Or specify log directory explicitly
  %(prog)s --url ws://127.0.0.1:9944 --log-dir ../../logs/from_2026-01-26_16-20-00_to_2026-01-26_16-40-00 \\
    --time-range '{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}'
  
  # Specify output directory
  %(prog)s --url ws://127.0.0.1:9944 --latest-n 50 --output-dir ./my_analysis
        """
    )
    
    parser.add_argument("--url", 
                       default="ws://127.0.0.1:9944",
                       help="WebSocket URL of the Substrate node (default: ws://127.0.0.1:9944)")
    
    parser.add_argument("--start-block", 
                       type=int,
                       help="Starting block number")
    
    parser.add_argument("--end-block", 
                       type=int,
                       help="Ending block number")
    
    parser.add_argument("--latest-n", 
                       type=int,
                       help="Fetch the latest N blocks")
    
    parser.add_argument("--time-range",
                       help='Time range as JSON to derive block numbers from logs, e.g., \'{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}\'')
    
    parser.add_argument("--from-time",
                       help="Start time (ISO 8601 or YYYY-MM-DD HH:MM:SS)")
    
    parser.add_argument("--to-time",
                       help="End time (ISO 8601 or YYYY-MM-DD HH:MM:SS)")
    
    parser.add_argument("--log-dir",
                       help="Directory containing log files (optional - will auto-discover or download if not specified)")
    
    parser.add_argument("--config",
                       help="Config file for download_logs.py (default: ../../secrets/substrate/performance/performance.json)")
    
    parser.add_argument("--node",
                       help="Specific node to use for block range extraction (default: all nodes)")
    
    parser.add_argument("--output-dir", 
                       help="Output directory for data and graphs (default: auto-generated based on block range)")
    
    parser.add_argument("--skip-fetch", 
                       action="store_true",
                       help="Skip fetching and use existing CSV file")
    
    parser.add_argument("--csv-file", 
                       help="Path to existing CSV file (required if --skip-fetch is used)")
    
    args = parser.parse_args()
    
    script_dir = Path(__file__).parent.absolute()
    
    # Auto-infer node name from URL if not specified
    if not args.node and args.url:
        # Extract node name from URL (e.g., george.node.sc.iog.io -> george)
        import re
        url_match = re.search(r'//([a-z]+)\.', args.url)
        if url_match:
            inferred_node = url_match.group(1)
            args.node = inferred_node
            print(f"Auto-detected node from URL: {inferred_node}")
            print()
    
    # If time range is specified, derive block numbers from logs
    if args.time_range or args.from_time or args.to_time:
        # Parse time range to determine log directory name if not specified
        if not args.log_dir:
            # Extract from and to times
            if args.time_range:
                try:
                    time_data = json.loads(args.time_range)
                    from_time = time_data.get('from')
                    to_time = time_data.get('to')
                except json.JSONDecodeError as e:
                    print(f"Error: Invalid JSON in --time-range: {e}")
                    sys.exit(1)
            else:
                from_time = args.from_time
                to_time = args.to_time
            
            if not from_time or not to_time:
                print("Error: Either --time-range or both --from-time and --to-time must be specified")
                sys.exit(1)
            
            # Generate expected log directory name
            try:
                # Parse times to create directory name format
                def parse_dt(time_str):
                    if 'T' in time_str or 'Z' in time_str:
                        time_str = time_str.replace('Z', '+00:00')
                        return datetime.fromisoformat(time_str)
                    else:
                        return datetime.strptime(time_str, '%Y-%m-%d %H:%M:%S')
                
                start_dt = parse_dt(from_time)
                end_dt = parse_dt(to_time)
                start_str = start_dt.strftime("%Y-%m-%d_%H-%M-%S")
                end_str = end_dt.strftime("%Y-%m-%d_%H-%M-%S")
                expected_log_dir = script_dir / ".." / ".." / "logs" / f"from_{start_str}_to_{end_str}"
                expected_log_dir = expected_log_dir.resolve()
            except Exception as e:
                print(f"Error parsing time range: {e}")
                sys.exit(1)
            
            # Check if log directory exists
            if expected_log_dir.exists() and list(expected_log_dir.glob("*.txt")):
                args.log_dir = str(expected_log_dir)
                print(f"Found existing logs: {args.log_dir}")
            else:
                # Download logs
                print(f"Logs not found at {expected_log_dir}")
                print("Downloading logs...")
                print("-" * 60)
                
                # Determine config file path
                if args.config:
                    config_file = args.config
                else:
                    config_file = str(script_dir / ".." / ".." / ".." / ".." / "secrets" / "substrate" / "performance" / "performance.json")
                
                download_script = script_dir / ".." / ".." / "download_logs.py"
                download_cmd = [
                    sys.executable,
                    str(download_script),
                    "--config", config_file
                ]
                
                if args.time_range:
                    download_cmd.extend(["--time-range", args.time_range])
                else:
                    download_cmd.extend(["--from-time", args.from_time, "--to-time", args.to_time])
                
                try:
                    subprocess.run(download_cmd, check=True)
                    args.log_dir = str(expected_log_dir)
                    print(f"Logs downloaded to: {args.log_dir}")
                    print()
                except subprocess.CalledProcessError as e:
                    print(f"\nError downloading logs: {e}")
                    print("\nTip: You can manually specify --log-dir if logs are in a different location")
                    sys.exit(1)
        
        print("Extracting block range from logs...")
        print("-" * 60)
        
        # Run extract_block_range_from_logs.py
        extract_cmd = [
            sys.executable,
            str(script_dir / "extract_block_range_from_logs.py"),
            "--log-dir", args.log_dir,
            "--json"
        ]
        
        if args.time_range:
            extract_cmd.extend(["--time-range", args.time_range])
        elif args.from_time and args.to_time:
            extract_cmd.extend(["--from-time", args.from_time, "--to-time", args.to_time])
        else:
            print("Error: Either --time-range or both --from-time and --to-time must be specified")
            sys.exit(1)
        
        if args.node:
            extract_cmd.extend(["--node", args.node])
        
        try:
            result = subprocess.run(extract_cmd, check=True, capture_output=True, text=True)
            block_range = json.loads(result.stdout)
            args.start_block = block_range["start_block"]
            args.end_block = block_range["end_block"]
            print(f"Found block range: {args.start_block} to {args.end_block}")
            print()
        except subprocess.CalledProcessError as e:
            print(f"\nError extracting block range: {e}")
            if e.stderr:
                print(e.stderr)
            sys.exit(1)
        except json.JSONDecodeError as e:
            print(f"\nError parsing block range output: {e}")
            sys.exit(1)
    
    # Determine output directory
    if args.output_dir:
        output_dir = Path(args.output_dir)
    else:
        # Auto-generate directory name
        if args.latest_n:
            dir_name = f"block_size_analysis_latest_{args.latest_n}"
        elif args.start_block is not None and args.end_block is not None:
            dir_name = f"block_size_analysis_{args.start_block}_to_{args.end_block}"
        else:
            dir_name = "block_size_analysis"
        output_dir = script_dir / dir_name
    
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print("=" * 60)
    print("BLOCK SIZE ANALYSIS")
    print("=" * 60)
    print()
    
    block_data = []

    # Step 1: Fetch block data (unless skipped)
    if not args.skip_fetch:
        print("STEP 1: Fetching block size data...")
        print("-" * 60)
        
        try:
            # Connect to node
            substrate = fetch_block_sizes.connect_to_node(args.url)
            
            # Determine block range
            if args.latest_n:
                # Get current block
                latest_block = substrate.get_block()
                end_block = latest_block['header']['number']
                start_block = max(0, end_block - args.latest_n + 1)
                print(f"Fetching latest {args.latest_n} blocks (#{start_block} to #{end_block})")
            elif args.start_block is not None and args.end_block is not None:
                start_block = args.start_block
                end_block = args.end_block
            else:
                print("Error: Either --latest-n or both --start-block and --end-block must be specified")
                sys.exit(1)
            
            # Fetch block data in memory
            block_data = fetch_block_sizes.fetch_block_range(substrate, start_block, end_block)
            
            # Close connection
            substrate.close()
            
        except Exception as e:
            print(f"\nError fetching block data: {e}")
            sys.exit(1)
        
        print()
    else:
        if not args.csv_file:
            print("Error: --csv-file is required when using --skip-fetch")
            sys.exit(1)
        csv_file = Path(args.csv_file)
        if not csv_file.exists():
            print(f"Error: CSV file not found: {csv_file}")
            sys.exit(1)
        print(f"Loading from existing CSV file: {csv_file}")
        try:
            block_data = plot_block_sizes.load_csv_data(csv_file)
        except Exception as e:
            print(f"Error loading CSV data: {e}")
            sys.exit(1)
        print()
    
    # Step 2: Generate visualizations
    print("STEP 2: Generating visualizations...")
    print("-" * 60)
    
    try:
        plot_block_sizes.plot_block_sizes_over_time(block_data, output_dir)
        plot_block_sizes.plot_block_size_distribution(block_data, output_dir)
        plot_block_sizes.plot_block_size_vs_extrinsics(block_data, output_dir)
        plot_block_sizes.plot_combined_analysis(block_data, output_dir)
        
        # Generate and save report
        plot_block_sizes.generate_report(block_data, output_dir)
        
    except Exception as e:
        print(f"\nError generating visualizations: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
    
    print()
    print("=" * 60)
    print("ANALYSIS COMPLETE")
    print("=" * 60)
    print(f"\nAll outputs saved to: {output_dir}")
    print(f"\nGenerated files:")
    if not args.skip_fetch:
         print(f"  (Data kept in memory, not saved to CSV)")
    print(f"  • block_sizes_over_time.png - Line chart of block sizes")
    print(f"  • block_size_distribution.png - Histogram of block sizes")
    print(f"  • block_size_vs_extrinsics.png - Scatter plot")
    print(f"  • block_size_analysis_dashboard.png - Combined dashboard")
    print(f"  • block_size_report.txt - Statistical report")
    print(f"\nTo view the dashboard:")
    print(f"  open {output_dir / 'block_size_analysis_dashboard.png'}")


if __name__ == "__main__":
    main()
