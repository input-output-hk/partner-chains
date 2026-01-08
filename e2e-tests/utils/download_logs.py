#!/usr/bin/env python3
import argparse
import requests
import sys
import os
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

# Default list of nodes (20 nodes in the environment)
DEFAULT_NODES = [
    "alice",
    "bob",
    "charlie",
    "dave",
    "eve",
    "ferdie",
    "george",
    "henry",
    "iris",
    "jack",
    "kate",
    "leo",
    "mike",
    "nina",
    "oliver",
    "paul",
    "quinn",
    "rita",
    "sam",
    "tom"
]

def load_config(config_file):
    """Load and decrypt config file using sops if needed."""
    config_path = Path(config_file)
    if not config_path.exists():
        print(f"Error: Config file not found: {config_file}")
        sys.exit(1)
    
    try:
        # Try to decrypt with sops
        result = subprocess.run(
            ['sops', '-d', str(config_path)],
            capture_output=True,
            text=True,
            check=True
        )
        config = json.loads(result.stdout)
        return config
    except subprocess.CalledProcessError:
        # If sops fails, try loading as plain JSON
        try:
            with open(config_path, 'r') as f:
                return json.load(f)
        except Exception as e:
            print(f"Error loading config file: {e}")
            sys.exit(1)
    except FileNotFoundError:
        print("Error: sops not found. Please install sops: brew install sops")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error parsing config JSON: {e}")
        sys.exit(1)

def parse_time_to_ns(time_str):
    """Parses an ISO 8601 time string to nanoseconds from epoch."""
    try:
        # Handles Z for UTC
        if time_str.endswith("Z"):
            time_str = time_str[:-1] + "+00:00"
        
        # Basic ISO parsing
        dt = datetime.fromisoformat(time_str)
        
        # Check if timezone aware, if not assume UTC
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
            
        return int(dt.timestamp() * 1e9)
    except ValueError as e:
        print(f"Error parsing time '{time_str}': {e}")
        print("Please use ISO 8601 format, e.g., '2023-01-01T12:00:00Z'")
        sys.exit(1)

def query_loki(url, query, start_ns, end_ns, limit=5000, headers=None):
    """Generator that yields log lines from Loki."""
    api_endpoint = f"{url.rstrip('/')}/loki/api/v1/query_range"
    
    current_start = start_ns
    
    while True:
        params = {
            'query': query,
            'start': current_start,
            'end': end_ns,
            'limit': limit,
            'direction': 'FORWARD'
        }
        
        try:
            response = requests.get(api_endpoint, params=params, headers=headers)
            response.raise_for_status()
            data = response.json()
        except requests.exceptions.RequestException as e:
            print(f"Request failed: {e}")
            if hasattr(e.response, 'text'):
                 print(f"Response body: {e.response.text[:200]}...")
            sys.exit(1)
        except ValueError as e:
            print(f"Failed to parse JSON response: {e}")
            print(f"Response content: {response.text[:200]}...")
            sys.exit(1)
            
        status = data.get('status')
        if status != 'success':
            print(f"Loki query failed status: {status}")
            sys.exit(1)
            
        result_type = data['data']['resultType']
        results = data['data']['result']
        
        if not results:
            break
            
        # For streams, we might have multiple streams matching the query.
        # But usually specific selector maps to one stream or we merge them.
        # We will iterate over all streams and merge them locally or just print them.
        # However, pagination with multiple streams is tricky.
        # If we query by specific host, we expect one stream or consistent streams.
        
        # Flatten all values from all streams
        all_values = []
        for stream in results:
            all_values.extend(stream['values'])
            
        # Sort by timestamp (the first element of the value array)
        # value is [timestamp_ns, log_line]
        all_values.sort(key=lambda x: int(x[0]))
        
        if not all_values:
            break
            
        for ts, line in all_values:
            yield ts, line
            
        # Prepare next iteration
        last_ts = int(all_values[-1][0])
        
        # If we got fewer items than limit, we are likely done.
        # However, with multiple streams it's more complex.
        # But if total items < limit, we are definitely done for this range.
        if len(all_values) < limit:
            break
        
        # Move start to last_ts + 1 to avoid duplicates
        # Note: if many logs have the exact same nanosecond timestamp, this might skip some.
        # Loki cursor based pagination is safer but this is 'query_range'.
        current_start = last_ts + 1
        
        if current_start > end_ns:
            break

def main():
    parser = argparse.ArgumentParser(description="Download logs from Loki/Grafana.")
    parser.add_argument("--config", help="Path to encrypted config file (e.g., secrets/substrate/performance/performance.json)")
    parser.add_argument("--url", help="Loki API URL (overrides config file)")
    parser.add_argument("--from-time", required=True, dest="start_time", help="Start time (ISO 8601)")
    parser.add_argument("--to-time", required=True, dest="end_time", help="End time (ISO 8601)")
    
    # Node selection
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--node", action='append', help="Specific node name (can be used multiple times)")
    group.add_argument("--nodes-file", help="File containing list of nodes (one per line)")
    
    parser.add_argument("--label", default="host", help="Loki label to filter by (default: host)")
    parser.add_argument("--header", action='append', help="Custom header 'Key: Value'. Can be used multiple times (overrides config file).")
    parser.add_argument("--output-dir", dest="output_dir", default="logs", help="Base output directory for log files (default: logs)")
    
    args = parser.parse_args()
    
    # Load config file if provided
    config = None
    if args.config:
        config = load_config(args.config)
    
    # Determine URL (command line overrides config)
    url = args.url
    if not url and config and 'grafana' in config:
        url = config['grafana'].get('url')
    if not url:
        url = "http://localhost:3100"
    
    # Parse headers (command line overrides config)
    headers = {}
    if config and 'grafana' in config and 'token' in config['grafana']:
        headers['Authorization'] = f"Bearer {config['grafana']['token']}"
    
    if args.header:
        for h in args.header:
            if ':' in h:
                key, value = h.split(':', 1)
                headers[key.strip()] = value.strip()
            else:
                print(f"Warning: Ignoring invalid header format '{h}'")

    start_ns = parse_time_to_ns(args.start_time)
    end_ns = parse_time_to_ns(args.end_time)
    
    nodes = []
    if args.node:
        nodes = args.node
    elif args.nodes_file:
        try:
            with open(args.nodes_file, 'r') as f:
                nodes = [line.strip() for line in f if line.strip()]
        except Exception as e:
            print(f"Error reading nodes file: {e}")
            sys.exit(1)
    else:
        nodes = DEFAULT_NODES
        print(f"No nodes specified, using default list: {', '.join(nodes)}")
    
    # Generate timestamp for the run
    run_timestamp = datetime.now(timezone.utc).strftime("%Y_%m_%d_%H_%M_%S")
    
    # Create timestamped output directory
    base_output_dir = Path(args.output_dir)
    output_dir = base_output_dir / run_timestamp
    output_dir.mkdir(parents=True, exist_ok=True)
    print(f"Output directory: {output_dir}")
    
    # Create log_run_details file with command parameters
    run_details = {
        "run_timestamp": run_timestamp,
        "start_time": args.start_time,
        "end_time": args.end_time,
        "nodes": nodes,
        "url": url,
        "label": args.label,
        "output_dir": str(output_dir)
    }
    
    details_file = output_dir / "log_run_details.json"
    try:
        with open(details_file, 'w', encoding='utf-8') as f:
            json.dump(run_details, f, indent=2)
        print(f"Run details saved to: {details_file}")
    except Exception as e:
        print(f"Warning: Failed to save run details: {e}")
        
    print(f"Downloading logs from {url}")
    print(f"Time range: {args.start_time} to {args.end_time}")
    
    for node in nodes:
        print(f"Processing node: {node}...")
        query = f'{{{args.label}="{node}"}}'
        output_filename = output_dir / f"{node}.txt"
        
        count = 0
        try:
            with open(output_filename, 'w', encoding='utf-8') as f:
                for _, line in query_loki(url, query, start_ns, end_ns, headers=headers):
                    f.write(line + "\n")
                    count += 1
            print(f"  Saved {count} lines to {output_filename}")
        except Exception as e:
            print(f"  Error processing node {node}: {e}")

if __name__ == "__main__":
    main()
