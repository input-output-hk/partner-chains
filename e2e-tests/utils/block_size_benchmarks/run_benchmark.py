#!/usr/bin/env python3

import argparse
import subprocess
import sys
import json
import os
from pathlib import Path


def run_command(cmd, cwd=None, capture_output=False):
    """Run a shell command and handle errors."""
    try:
        if capture_output:
            result = subprocess.run(
                cmd,
                cwd=cwd,
                capture_output=True,
                text=True,
                check=True
            )
            return result.stdout
        else:
            subprocess.run(cmd, cwd=cwd, check=True)
            return None
    except subprocess.CalledProcessError as e:
        print(f"Error running command: {' '.join(cmd)}")
        if e.stderr:
            print(f"Error output: {e.stderr}")
        sys.exit(1)


def download_logs(download_script, args):
    """Download logs using download_logs.py and return the output directory."""
    cmd = [sys.executable, str(download_script)]
    
    # Add config if provided
    if args.config:
        cmd.extend(["--config", args.config])
    
    # Add URL if provided
    if args.url:
        cmd.extend(["--url", args.url])
    
    # Add time range
    cmd.extend(["--from-time", args.from_time])
    cmd.extend(["--to-time", args.to_time])
    
    # Add nodes
    if args.node:
        for node in args.node:
            cmd.extend(["--node", node])
    elif args.nodes_file:
        cmd.extend(["--nodes-file", args.nodes_file])
    
    # Add headers if provided
    if args.header:
        for header in args.header:
            cmd.extend(["--header", header])
    
    # Add output directory
    output_base = args.output_dir if args.output_dir else "logs"
    cmd.extend(["--output-dir", output_base])
    
    print("Downloading logs...")
    print(f"Running: {' '.join(cmd)}\n")
    run_command(cmd)
    
    # Find the most recent timestamped directory
    base_path = Path(output_base)
    if not base_path.exists():
        print(f"Error: Output directory {base_path} does not exist")
        sys.exit(1)
    
    # Look for the most recent timestamped subdirectory
    subdirs = [d for d in base_path.iterdir() if d.is_dir()]
    if not subdirs:
        print(f"Error: No subdirectories found in {base_path}")
        sys.exit(1)
    
    # Sort by modification time and get the most recent
    latest_dir = max(subdirs, key=lambda d: d.stat().st_mtime)
    
    print(f"Logs downloaded to: {latest_dir}\n")
    return latest_dir


def extract_nodes_from_log_dir(log_dir):
    """Extract node names from log files in the directory."""
    nodes = []
    for file in log_dir.glob("*.txt"):
        if file.name != "block_propagation_report.txt" and file.name != "log_run_details.json":
            node_name = file.stem  # Get filename without extension
            nodes.append(node_name)
    return sorted(nodes)


def extract_nodes_from_details(log_dir):
    """Extract nodes from log_run_details.json if it exists."""
    details_file = log_dir / "log_run_details.json"
    if details_file.exists():
        try:
            with open(details_file, 'r') as f:
                details = json.load(f)
                return details.get('nodes', [])
        except Exception as e:
            print(f"Warning: Could not read log_run_details.json: {e}")
    return []


def run_extractor(script_dir, log_dir, nodes):
    """Run the extractor.py script in the log directory."""
    extractor_script = script_dir / "extractor.py"
    
    cmd = [sys.executable, str(extractor_script)] + nodes
    
    print(f"Running extractor in {log_dir}...")
    print(f"Analyzing nodes: {', '.join(nodes)}\n")
    run_command(cmd, cwd=log_dir)
    
    report_file = log_dir / "block_propagation_report.txt"
    if not report_file.exists():
        print("Error: block_propagation_report.txt was not created")
        sys.exit(1)
    
    print(f"Block propagation report created: {report_file}\n")
    return report_file


def run_analyzer(script_dir, log_dir, report_file, nodes):
    """Run the analyzer.py script to generate statistics."""
    analyzer_script = script_dir / "analyzer.py"
    analysis_output = log_dir / "analysis.txt"
    
    cmd = [sys.executable, str(analyzer_script), str(report_file), str(analysis_output)] + nodes
    
    print(f"Running analyzer...")
    run_command(cmd)
    
    if not analysis_output.exists():
        print("Error: analysis.txt was not created")
        sys.exit(1)
    
    print(f"Analysis complete: {analysis_output}\n")
    return analysis_output


def main():
    parser = argparse.ArgumentParser(
        description="Run complete block size benchmarking workflow: download logs, extract data, and analyze."
    )
    
    # Download logs arguments
    parser.add_argument("--config", help="Path to encrypted config file")
    parser.add_argument("--url", help="Loki API URL (overrides config file)")
    parser.add_argument("--from-time", required=True, help="Start time (ISO 8601)")
    parser.add_argument("--to-time", required=True, help="End time (ISO 8601)")
    
    # Node selection
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--node", action='append', help="Specific node name (can be used multiple times)")
    group.add_argument("--nodes-file", help="File containing list of nodes (one per line)")
    
    parser.add_argument("--header", action='append', help="Custom header 'Key: Value'. Can be used multiple times.")
    parser.add_argument("--output-dir", help="Base output directory for log files (default: logs)")
    
    # Workflow control
    parser.add_argument("--skip-download", action="store_true", 
                       help="Skip log download and use existing logs in --log-dir")
    parser.add_argument("--log-dir", help="Path to existing log directory (required if --skip-download is used)")
    
    args = parser.parse_args()
    
    # Validate skip-download arguments
    if args.skip_download and not args.log_dir:
        print("Error: --log-dir is required when using --skip-download")
        sys.exit(1)
    
    # Get script directory
    script_dir = Path(__file__).parent.absolute()
    download_script = script_dir.parent / "download_logs.py"
    
    # Check if download script exists
    if not download_script.exists() and not args.skip_download:
        print(f"Error: download_logs.py not found at {download_script}")
        sys.exit(1)
    
    # Step 1: Download logs (or use existing)
    if args.skip_download:
        log_dir = Path(args.log_dir)
        if not log_dir.exists():
            print(f"Error: Log directory does not exist: {log_dir}")
            sys.exit(1)
        print(f"Using existing logs in: {log_dir}\n")
    else:
        log_dir = download_logs(download_script, args)
    
    # Step 2: Determine which nodes to analyze
    # Priority: log_run_details.json > log files > command-line arguments
    nodes = extract_nodes_from_details(log_dir)
    if not nodes:
        nodes = extract_nodes_from_log_dir(log_dir)
    if not nodes:
        print("Error: No log files found in the log directory")
        sys.exit(1)
    
    print(f"Nodes to analyze: {', '.join(nodes)}\n")
    
    # Step 3: Run extractor
    report_file = run_extractor(script_dir, log_dir, nodes)
    
    # Step 4: Run analyzer
    analysis_file = run_analyzer(script_dir, log_dir, report_file, nodes)
    
    # Summary
    print("=" * 60)
    print("BENCHMARKING COMPLETE")
    print("=" * 60)
    print(f"Log directory:      {log_dir}")
    print(f"Propagation report: {report_file}")
    print(f"Analysis:           {analysis_file}")
    print()
    print("To view the analysis:")
    print(f"  cat {analysis_file}")


if __name__ == "__main__":
    main()
