#!/usr/bin/env python3
import subprocess
import json
import sys
import os
import time
import shutil
import tempfile
import concurrent.futures
import argparse
import random

# Configuration
TOOLKIT_CMD = "midnight-node-toolkit"
REMOTE_RELAYS = [
    "ferdie",
    "george",
    "henry",
    "iris",
    "jack",
    "paul",
    "quinn",
    "rita",
    "sam",
    "tom"
]
LOCAL_RELAYS = [
    "ws://localhost:9933",
    "ws://localhost:9934",
    "ws://localhost:9935",
    "ws://localhost:9936",
    "ws://localhost:9937",
]
RELAYS = REMOTE_RELAYS
START_INDEX = 1
END_INDEX = 50
DB_PATH = "toolkit.db"
NODE_URL = "ws://ferdie.node.sc.iog.io:9944" # "ws://localhost:9944"
DELAY = 0.25


def get_balance(index, node_url_pattern, verbose=False):
    """Gets the balance for a given seed index."""
    time.sleep(random.uniform(DELAY * 0.5, DELAY * 1.5))
    seed = f"{index:064}"

    relay_name = RELAYS[index % len(RELAYS)]
    if relay_name.startswith("ws://") or relay_name.startswith("wss://"):
        node_url = relay_name
    elif "ferdie" in node_url_pattern:
        node_url = node_url_pattern.replace("ferdie", relay_name)
    else:
        node_url = node_url_pattern

    cmd = [
        TOOLKIT_CMD, "show-wallet",
        "--seed", seed,
        "--src-url", node_url
    ]

    if verbose:
        print(f"[{index}] Running: {' '.join(cmd)}")

    try:
        with tempfile.TemporaryDirectory() as temp_dir:
            # Copy toolkit.db to temp_dir to avoid locking
            db_copy_start = time.time()
            if os.path.exists(DB_PATH):
                shutil.copy(DB_PATH, os.path.join(temp_dir, "toolkit.db"))
            db_copy_time = time.time() - db_copy_start

            exec_start = time.time()
            result = subprocess.run(cmd, capture_output=True, text=True, check=True, cwd=temp_dir)
            exec_time = time.time() - exec_start

        output = result.stdout

        if verbose:
            if result.stdout:
                print(f"[{index}] STDOUT: {result.stdout.strip()}")
            if result.stderr:
                print(f"[{index}] STDERR: {result.stderr.strip()}")

        # Mimic sed -n '/^{/,$p': Find lines starting from the first one that begins with '{'
        lines = output.splitlines()
        json_lines = []
        capture = False
        for line in lines:
            if line.strip().startswith('{'):
                capture = True
            if capture:
                json_lines.append(line)

        if not json_lines:
            print(f"⚠️  Seed {index}: No JSON output found.")
            return 0

        json_str = "\n".join(json_lines)
        data = json.loads(json_str)

        # Mimic jq '.utxos[]?.value' | jq -s 'add'
        utxos = data.get("utxos") or []

        total_balance = sum(int(utxo.get("value", 0)) for utxo in utxos)
        print(f"Seed {index:4}: {total_balance:<28} [DB Copy: {db_copy_time:.4f}s, Exec: {exec_time:.4f}s]")
        return total_balance

    except subprocess.CalledProcessError as e:
        print(f"❌ Error getting balance for seed {index}: {e.stderr.strip()}")
        return 0
    except json.JSONDecodeError:
        print(f"❌ Failed to parse JSON for seed {index}")
        return 0
    except Exception as e:
        print(f"❌ Unexpected error for seed {index}: {e}")
        return 0

def main():
    parser = argparse.ArgumentParser(description="Check wallet balances.")
    parser.add_argument("-s", "--dest-start", type=int, default=START_INDEX, help="Starting seed index")
    parser.add_argument("-e", "--dest-end", type=int, default=END_INDEX, help="Ending seed index")
    parser.add_argument("-i", "--dest-indices", nargs='+', help="List of specific seed indices (space or comma-separated, overrides --dest-start/--dest-end)")
    parser.add_argument("-v", "--verbose", action="store_true", help="Enable verbose output")
    parser.add_argument("--node-url", type=str, default=NODE_URL, help="Node URL. 'ferdie' will be replaced by relay names if present.")
    args = parser.parse_args()

    if args.dest_indices:
        target_indices = []
        for item in args.dest_indices:
            try:
                target_indices.extend([int(i.strip()) for i in item.split(',') if i.strip()])
            except ValueError:
                print(f"❌ Error: Invalid value in --dest-indices: '{item}'. Please provide a list of integers.")
                sys.exit(1)
    else:
        target_indices = list(range(args.dest_start, args.dest_end + 1))

    global DB_PATH
    if not os.path.exists(DB_PATH):
        print(f"⚠️  Warning: '{DB_PATH}' not found in current directory.")
        user_input = input("Please enter the full path to toolkit.db: ").strip()
        if not user_input:
            print("❌ No path provided. Exiting.")
            sys.exit(1)

        DB_PATH = user_input
        if not os.path.exists(DB_PATH):
            print(f"❌ Error: File '{DB_PATH}' not found.")
            sys.exit(1)

    start_time = time.time()
    print(f"🚀 Checking night balances for {len(target_indices)} seeds across {len(RELAYS)} nodes...")

    total_wallets = len(target_indices)
    cpu_count = os.cpu_count() or 1
    max_threads = max(1, int(cpu_count * 0.9))
    max_workers = min(total_wallets, max_threads)
    print(f"ℹ️  Using {max_workers} threads.")

    total_sum = 0
    empty_seeds = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_to_index = {executor.submit(get_balance, i, args.node_url, verbose=args.verbose): i for i in target_indices}
        for future in concurrent.futures.as_completed(future_to_index):
            index = future_to_index[future]
            balance = future.result()
            total_sum += balance
            if balance == 0:
                empty_seeds.append(index)

    end_time = time.time()
    print(f"\n💰 Total Night Balance: {total_sum}")
    if empty_seeds:
        print(f"Empty wallet seeds: {sorted(empty_seeds)}")
    print(f"⏱️ Total execution time: {end_time - start_time:.2f} seconds")

if __name__ == "__main__":
    main()