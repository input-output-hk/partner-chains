#!/usr/bin/env python3

import argparse
import subprocess
import sys
import json
import os
import re
from pathlib import Path
from datetime import datetime


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
    
    # Add output directory - if not specified, download_logs.py will use script_dir/logs
    if args.output_dir:
        cmd.extend(["--output-dir", args.output_dir])
    
    print("Downloading logs...")
    print(f"Running: {' '.join(cmd)}\n")
    run_command(cmd)
    
    # Construct expected directory name based on date range
    # This matches the logic in download_logs.py
    try:
        from datetime import datetime
        # Handle both space-separated and T-separated formats
        from_time_normalized = args.from_time.replace(' ', 'T').replace('Z', '+00:00')
        to_time_normalized = args.to_time.replace(' ', 'T').replace('Z', '+00:00')
        start_dt = datetime.fromisoformat(from_time_normalized)
        end_dt = datetime.fromisoformat(to_time_normalized)
        start_str = start_dt.strftime("%Y-%m-%d_%H-%M-%S")
        end_str = end_dt.strftime("%Y-%m-%d_%H-%M-%S")
        date_range_folder = f"from_{start_str}_to_{end_str}"
    except Exception as e:
        # Fallback to simple names
        start_str = args.from_time.replace(':', '-').replace('T', '_').replace(' ', '_')
        end_str = args.to_time.replace(':', '-').replace('T', '_').replace(' ', '_')
        date_range_folder = f"from_{start_str}_to_{end_str}"
    
    # Determine base path
    if args.output_dir:
        base_path = Path(args.output_dir)
    else:
        # download_logs.py will use its own script_dir/logs
        base_path = download_script.parent / "logs"
    
    # Expected log directory
    log_dir = base_path / date_range_folder
    
    if not log_dir.exists():
        print(f"Error: Expected log directory does not exist: {log_dir}")
        sys.exit(1)
    
    print(f"Using logs in: {log_dir}\n")
    return log_dir


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


def parse_block_propagation_report(report_file):
    """Parse the block_propagation_report.txt file and extract import time data.
    
    Returns:
        dict: Dictionary mapping node names to lists of (block_number, import_time_ms) tuples
    """
    node_data = {}
    current_block_number = None
    
    with open(report_file, 'r') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            
            # Match block header: Block #45553 ...
            block_match = re.match(r'^Block #(\d+)', line)
            if block_match:
                current_block_number = int(block_match.group(1))
                continue
            
            # Match import lines: "  Imported by <node> after <time> ms at <timestamp>"
            # or creator node: "  Imported by <node> (creator node) at <timestamp>"
            import_match = re.match(r'^Imported by (\w+)', line)
            if import_match and current_block_number is not None:
                node_name = import_match.group(1)
                
                # Check if this is a creator node (no "after X ms" part)
                if '(creator node)' in line:
                    # Skip creator nodes as they have no import time
                    continue
                
                # Extract import time in ms
                time_match = re.search(r'after ([\d.]+) ms', line)
                if time_match:
                    import_time = float(time_match.group(1))
                    
                    if node_name not in node_data:
                        node_data[node_name] = []
                    
                    node_data[node_name].append((current_block_number, import_time))
    
    return node_data


def parse_block_creation_times(report_file):
    """Parse the block_propagation_report.txt file and extract block creation times.
    
    Block creation time is measured from the slot start (0, 6, 12, 18, 24, 30... seconds)
    to when the block is actually created/sealed.
    
    Returns:
        dict: Dictionary mapping node names to lists of (block_number, creation_time_ms) tuples
    """
    from datetime import datetime
    
    creation_data = {}
    current_block_number = None
    current_creator = None
    
    with open(report_file, 'r') as f:
        for line in f:
            line = line.strip()
            if not line:
                # Reset for next block
                current_block_number = None
                current_creator = None
                continue
            
            # Match block header: Block #45553 ...
            block_match = re.match(r'^Block #(\d+)', line)
            if block_match:
                current_block_number = int(block_match.group(1))
                continue
            
            # Match creator line: "Created by: <node> at <timestamp>"
            creator_match = re.match(r'^Created by: (\w+) at (.+)', line)
            if creator_match and current_block_number is not None:
                current_creator = creator_match.group(1)
                creation_time_str = creator_match.group(2)
                creation_time = datetime.strptime(creation_time_str, "%Y-%m-%d %H:%M:%S.%f")
                
                # Calculate time from slot start
                # Slots start at 0, 6, 12, 18, 24, 30, 36, 42, 48, 54 seconds
                # Find the most recent slot start before creation_time
                seconds = creation_time.second
                microseconds = creation_time.microsecond
                
                # Find the slot start (floor to nearest 6-second boundary)
                slot_start_second = (seconds // 6) * 6
                
                # Calculate time from slot start
                time_from_slot_start_ms = ((seconds - slot_start_second) * 1000 + 
                                          microseconds / 1000)
                
                if current_creator not in creation_data:
                    creation_data[current_creator] = []
                
                creation_data[current_creator].append((current_block_number, time_from_slot_start_ms))
    
    return creation_data


def generate_import_time_graph(report_file, output_dir):
    """Generate a graph showing import times over time for each node.
    
    Args:
        report_file: Path to block_propagation_report.txt
        output_dir: Directory to save the graph
    """
    try:
        import matplotlib
        matplotlib.use('Agg')  # Non-interactive backend
        import matplotlib.pyplot as plt
    except ImportError:
        print("Error: matplotlib not installed.")
        print("Please use the wrapper script: ./run_benchmark.sh")
        print("Or install manually: pip3 install -r requirements.txt")
        return None
    
    print("Generating import time graph...")
    
    # Parse the report file
    node_data = parse_block_propagation_report(report_file)
    
    if not node_data:
        print("Warning: No import data found in report file.")
        return None
    
    # Create figure
    plt.figure(figsize=(14, 8))
    
    # Define 20 distinct colors for the nodes
    colors = [
        '#1f77b4', '#ff7f0e', '#2ca02c', '#d62728', '#9467bd',
        '#8c564b', '#e377c2', '#7f7f7f', '#bcbd22', '#17becf',
        '#aec7e8', '#ffbb78', '#98df8a', '#ff9896', '#c5b0d5',
        '#c49c94', '#f7b6d2', '#c7c7c7', '#dbdb8d', '#9edae5'
    ]
    
    # Sort nodes by name for consistent ordering
    sorted_nodes = sorted(node_data.keys())
    
    # Plot each node's data
    for idx, node_name in enumerate(sorted_nodes):
        data = node_data[node_name]
        if not data:
            continue
        
        block_numbers = [point[0] for point in data]
        import_times = [point[1] for point in data]
        
        color = colors[idx % len(colors)]
        plt.plot(block_numbers, import_times, 
                marker='o', markersize=3, linewidth=1.5,
                label=node_name, color=color, alpha=0.7)
    
    # Configure plot
    plt.xlabel('Block Number', fontsize=12)
    plt.ylabel('Import Time (ms)', fontsize=12)
    plt.title('Block Import Times Over Time by Node', fontsize=14, fontweight='bold')
    plt.grid(True, alpha=0.3, linestyle='--')
    plt.legend(bbox_to_anchor=(1.05, 1), loc='upper left', 
              fontsize=9, ncol=1 if len(sorted_nodes) <= 20 else 2)
    plt.tight_layout()
    
    # Save the graph
    graph_file = output_dir / "block_import_times_graph.png"
    plt.savefig(graph_file, dpi=150, bbox_inches='tight')
    plt.close()
    
    print(f"Graph saved: {graph_file}\n")
    return graph_file


def generate_block_creation_graph(report_file, output_dir):
    """Generate a bar chart showing block creation times.
    
    Args:
        report_file: Path to block_propagation_report.txt
        output_dir: Directory to save the graph
    """
    try:
        import matplotlib
        matplotlib.use('Agg')  # Non-interactive backend
        import matplotlib.pyplot as plt
        import numpy as np
    except ImportError:
        print("Error: matplotlib not installed.")
        print("Please use the wrapper script: ./run_benchmark.sh")
        print("Or install manually: pip3 install -r requirements.txt")
        return None
    
    print("Generating block creation time graph...")
    
    # Parse the report file
    creation_data = parse_block_creation_times(report_file)
    
    if not creation_data:
        print("Warning: No block creation data found in report file.")
        return None
    
    # Flatten all data points into (block_number, node_name, creation_time) tuples
    all_points = []
    for node_name, data in creation_data.items():
        for block_num, creation_time in data:
            all_points.append((block_num, node_name, creation_time))
    
    # Sort by block number
    all_points.sort(key=lambda x: x[0])
    
    if not all_points:
        print("Warning: No block creation data points found.")
        return None
    
    # Extract data for plotting
    block_numbers = [p[0] for p in all_points]
    node_names = [p[1] for p in all_points]
    creation_times = [p[2] for p in all_points]
    
    # Create color map for nodes
    colors_palette = [
        '#1f77b4', '#ff7f0e', '#2ca02c', '#d62728', '#9467bd',
        '#8c564b', '#e377c2', '#7f7f7f', '#bcbd22', '#17becf',
        '#aec7e8', '#ffbb78', '#98df8a', '#ff9896', '#c5b0d5',
        '#c49c94', '#f7b6d2', '#c7c7c7', '#dbdb8d', '#9edae5'
    ]
    unique_nodes = sorted(set(node_names))
    node_color_map = {node: colors_palette[i % len(colors_palette)] for i, node in enumerate(unique_nodes)}
    bar_colors = [node_color_map[node] for node in node_names]
    
    # Create figure
    fig, ax = plt.subplots(figsize=(14, 8))
    
    # Create bar chart
    x_positions = np.arange(len(all_points))
    bars = ax.bar(x_positions, creation_times, color=bar_colors, alpha=0.7, edgecolor='black', linewidth=0.5)
    
    # Set x-axis labels to block numbers (show every nth label to avoid crowding)
    label_frequency = max(1, len(block_numbers) // 20)
    ax.set_xticks(x_positions[::label_frequency])
    ax.set_xticklabels([str(bn) for bn in block_numbers[::label_frequency]], rotation=45, ha='right')
    
    # Configure plot
    ax.set_xlabel('Block Number', fontsize=12)
    ax.set_ylabel('Block Creation Time (ms)', fontsize=12)
    ax.set_title('Block Creation Times by Node', fontsize=14, fontweight='bold')
    ax.grid(True, alpha=0.3, linestyle='--', axis='y')
    
    # Create legend
    legend_handles = [plt.Rectangle((0,0),1,1, color=node_color_map[node], alpha=0.7) for node in unique_nodes]
    ax.legend(legend_handles, unique_nodes, bbox_to_anchor=(1.05, 1), loc='upper left', fontsize=9)
    
    plt.tight_layout()
    
    # Save the graph
    graph_file = output_dir / "block_creation_times_graph.png"
    plt.savefig(graph_file, dpi=150, bbox_inches='tight')
    plt.close()
    
    print(f"Graph saved: {graph_file}\n")
    return graph_file


def generate_combined_graph(report_file, output_dir):
    """Generate a combined graph showing both block creation and import times.
    
    Args:
        report_file: Path to block_propagation_report.txt
        output_dir: Directory to save the graph
    """
    try:
        import matplotlib
        matplotlib.use('Agg')  # Non-interactive backend
        import matplotlib.pyplot as plt
        import numpy as np
    except ImportError:
        print("Error: matplotlib not installed.")
        print("Please use the wrapper script: ./run_benchmark.sh")
        print("Or install manually: pip3 install -r requirements.txt")
        return None
    
    print("Generating combined graph...")
    
    # Parse the report file
    import_data = parse_block_propagation_report(report_file)
    creation_data = parse_block_creation_times(report_file)
    
    if not import_data and not creation_data:
        print("Warning: No data found in report file.")
        return None
    
    # Create figure with 2 subplots
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(14, 12), sharex=True)
    
    # Define 20 distinct colors for the nodes
    colors = [
        '#1f77b4', '#ff7f0e', '#2ca02c', '#d62728', '#9467bd',
        '#8c564b', '#e377c2', '#7f7f7f', '#bcbd22', '#17becf',
        '#aec7e8', '#ffbb78', '#98df8a', '#ff9896', '#c5b0d5',
        '#c49c94', '#f7b6d2', '#c7c7c7', '#dbdb8d', '#9edae5'
    ]
    
    # Plot 1: Block Import Times (Network Propagation)
    if import_data:
        sorted_nodes = sorted(import_data.keys())
        for idx, node_name in enumerate(sorted_nodes):
            data = import_data[node_name]
            if not data:
                continue
            
            block_numbers = [point[0] for point in data]
            import_times = [point[1] for point in data]
            
            color = colors[idx % len(colors)]
            ax1.plot(block_numbers, import_times, 
                    marker='o', markersize=2, linewidth=1,
                    label=node_name, color=color, alpha=0.6)
        
        ax1.set_ylabel('Network Import Time (ms)', fontsize=11)
        ax1.set_title('Block Import Times (Network Propagation)', fontsize=12, fontweight='bold')
        ax1.grid(True, alpha=0.3, linestyle='--')
        ax1.legend(bbox_to_anchor=(1.05, 1), loc='upper left', fontsize=8, ncol=1)
    
    # Plot 2: Block Creation Times (Bar Chart)
    if creation_data:
        # Flatten all creation data points
        all_creation_points = []
        for node_name, data in creation_data.items():
            for block_num, creation_time in data:
                all_creation_points.append((block_num, node_name, creation_time))
        
        # Sort by block number
        all_creation_points.sort(key=lambda x: x[0])
        
        if all_creation_points:
            creation_block_numbers = [p[0] for p in all_creation_points]
            creation_node_names = [p[1] for p in all_creation_points]
            creation_times = [p[2] for p in all_creation_points]
            
            # Create color map for nodes (reuse colors from top panel)
            unique_creators = sorted(set(creation_node_names))
            node_color_map = {}
            for node in unique_creators:
                color_idx = sorted_nodes.index(node) if node in sorted_nodes else unique_creators.index(node)
                node_color_map[node] = colors[color_idx % len(colors)]
            
            bar_colors = [node_color_map[node] for node in creation_node_names]
            
            # Create bar chart
            x_positions = np.arange(len(all_creation_points))
            ax2.bar(x_positions, creation_times, color=bar_colors, alpha=0.7, edgecolor='black', linewidth=0.3)
            
            # Set x-axis labels (show every nth label to avoid crowding)
            label_frequency = max(1, len(creation_block_numbers) // 15)
            ax2.set_xticks(x_positions[::label_frequency])
            ax2.set_xticklabels([str(bn) for bn in creation_block_numbers[::label_frequency]], rotation=45, ha='right')
            
            # Create legend
            legend_handles = [plt.Rectangle((0,0),1,1, color=node_color_map[node], alpha=0.7) for node in unique_creators]
            ax2.legend(legend_handles, unique_creators, bbox_to_anchor=(1.05, 1), loc='upper left', fontsize=8)
        
        ax2.set_ylabel('Block Creation Time (ms)', fontsize=11)
        ax2.set_xlabel('Block Number', fontsize=11)
        ax2.set_title('Block Creation Times', fontsize=12, fontweight='bold')
        ax2.grid(True, alpha=0.3, linestyle='--', axis='y')
    
    plt.tight_layout()
    
    # Save the graph
    graph_file = output_dir / "combined_times_graph.png"
    plt.savefig(graph_file, dpi=150, bbox_inches='tight')
    plt.close()
    
    print(f"Combined graph saved: {graph_file}\n")
    return graph_file


def main():
    parser = argparse.ArgumentParser(
        description="Run complete block size benchmarking workflow: download logs, extract data, and analyze."
    )
    
    # Download logs arguments
    parser.add_argument("--config", 
                       default="../../../secrets/substrate/performance/performance.json",
                       help="Path to encrypted config file (default: ../../../secrets/substrate/performance/performance.json)")
    parser.add_argument("--url", help="Loki API URL (overrides config file)")
    parser.add_argument("--time-range", help='Time range as JSON, e.g., \'{"from":"2026-01-20 10:34:25","to":"2026-01-20 11:34:25"}\'')
    parser.add_argument("--from-time", help="Start time (ISO 8601 or YYYY-MM-DD HH:MM:SS)")
    parser.add_argument("--to-time", help="End time (ISO 8601 or YYYY-MM-DD HH:MM:SS)")
    
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
    
    # Validate that we have time range (unless skipping download)
    if not args.skip_download and (not args.from_time or not args.to_time):
        print("Error: Either --time-range or both --from-time and --to-time must be provided")
        sys.exit(1)
    
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
    
    # Step 5: Generate graphs
    import_graph = generate_import_time_graph(report_file, log_dir)
    creation_graph = generate_block_creation_graph(report_file, log_dir)
    combined_graph = generate_combined_graph(report_file, log_dir)
    
    # Summary
    print("=" * 60)
    print("BENCHMARKING COMPLETE")
    print("=" * 60)
    print(f"Log directory:      {log_dir}")
    print(f"Propagation report: {report_file}")
    print(f"Analysis:           {analysis_file}")
    if import_graph:
        print(f"Import time graph:  {import_graph}")
    if creation_graph:
        print(f"Creation time graph: {creation_graph}")
    if combined_graph:
        print(f"Combined graph:     {combined_graph}")
    print()
    print("To view the analysis:")
    print(f"  cat {analysis_file}")
    if combined_graph:
        print(f"To view the combined graph:")
        print(f"  open {combined_graph}")


if __name__ == "__main__":
    main()
