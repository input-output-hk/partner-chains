#!/usr/bin/env python3
import subprocess
import argparse
import re
import sys
import os
import shutil
import tempfile

# Default Configuration
DEFAULT_NODE_URL = "ws://henry.node.sc.iog.io:9944"
DEFAULT_TOOLKIT_CMD = "midnight-node-toolkit"

def run_command(cmd, cwd=None, verbose=False):
    """Runs a command and returns stdout and stderr combined."""
    if verbose:
        print(f"Running: {' '.join(cmd)}")
    try:
        # Capture both stdout and stderr as the log output might be in either
        result = subprocess.run(cmd, capture_output=True, text=True, check=True, cwd=cwd)
        # Combine outputs for searching
        output = result.stdout + "\n" + result.stderr
        if verbose:
            print(output)
        return output
    except subprocess.CalledProcessError as e:
        print(f"\n❌ Error executing command: {' '.join(cmd)}")
        print(f"STDOUT: {e.stdout}")
        print(f"STDERR: {e.stderr}")
        sys.exit(1)
    except FileNotFoundError:
        print(f"\n❌ Error: Executable '{cmd[0]}' not found. Ensure it is in your PATH.")
        sys.exit(1)

def main():
    parser = argparse.ArgumentParser(description="Deploy simple contract script.")
    parser.add_argument("--node-url", default=DEFAULT_NODE_URL, help="Node URL (source and dest)")
    parser.add_argument("--toolkit-cmd", default=DEFAULT_TOOLKIT_CMD, help="Path to midnight-node-toolkit")
    parser.add_argument("-v", "--verbose", action="store_true", help="Verbose output")

    args = parser.parse_args()

    print(f"🚀 Deploying simple contract to {args.node_url}...")

    cmd = [
        args.toolkit_cmd, "generate-txs",
        "contract-simple",
        "deploy",
        "-s", args.node_url,
        "-d", args.node_url
    ]

    # Use a temporary directory to avoid database locking issues
    with tempfile.TemporaryDirectory() as temp_dir:
        db_path = "toolkit.db"
        if os.path.exists(db_path):
            shutil.copy(db_path, os.path.join(temp_dir, "toolkit.db"))
        output = run_command(cmd, cwd=temp_dir, verbose=args.verbose)

    # Parse for contract address
    # Looking for: CONTRACT ADDRESS: ContractAddress(hex_string)
    match = re.search(r"CONTRACT ADDRESS: ContractAddress\(([a-fA-F0-9]+)\)", output)
    
    if match:
        contract_address = match.group(1)
        print(f"Contract Address: {contract_address}")
    else:
        print("\n❌ Could not find contract address in output.")
        if not args.verbose:
            print("Output:\n" + output)
        sys.exit(1)

if __name__ == "__main__":
    main()