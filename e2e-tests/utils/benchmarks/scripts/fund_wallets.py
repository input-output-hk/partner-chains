#!/usr/bin/env python3
import subprocess
import json
import sys
import time
import random
import concurrent.futures
import math
import shutil
import tempfile
import os
import argparse

# Configuration
TOOLKIT_CMD = "midnight-node-toolkit"
RELAYS = [
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
TARGET_START_INDEX = 4
TARGET_END_INDEX = 10
FUNDING_START_INDEX = 1
FUNDING_END_INDEX = 3
TOKEN_TYPE = "0000000000000000000000000000000000000000000000000000000000000000"
DB_PATH = "toolkit.db"
NODE_URL = "ws://ferdie.node.sc.iog.io:9944" # "ws://localhost:9944"
FUNDING_AMOUNT = 3000000
FUNDING_SEEDS = []

def run_command(cmd, cwd=None, verbose=False):
    """Runs a command and returns stdout if successful, exits otherwise."""
    if verbose:
        print(f"Running: {' '.join(cmd)}")
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True, cwd=cwd)
        if verbose:
            if result.stdout:
                print(f"STDOUT: {result.stdout.strip()}")
            if result.stderr:
                print(f"STDERR: {result.stderr.strip()}")
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        print(f"\n❌ Error executing command: {' '.join(cmd)}")
        print(f"STDOUT: {e.stdout}")
        print(f"STDERR: {e.stderr}")
        raise e
    except FileNotFoundError:
        print(f"\n❌ Error: Executable '{cmd[0]}' not found. Ensure it is in your PATH.")
        sys.exit(1)

def get_wallet_address(index, cwd=None, verbose=False):
    """Creates a wallet seed and retrieves its address."""
    # Seed format: 00..xx padded to 64 chars
    seed = f"{index:064}"

    cmd = [
        TOOLKIT_CMD, "show-address",
        "--network", "undeployed",
        "--seed", seed
    ]

    output = run_command(cmd, cwd=cwd, verbose=verbose)
    try:
        data = json.loads(output)
        return data["unshielded"]
    except json.JSONDecodeError:
        print(f"\n❌ Failed to parse JSON from show-address output: {output}")
        sys.exit(1)
    except KeyError:
        print(f"\n❌ JSON output does not contain 'unshielded' field: {output}")
        sys.exit(1)

def fund_address(address, funding_seed, node_url, cwd=None, verbose=False):
    """Funds the given address using the source seed."""

    cmd = [
        TOOLKIT_CMD, "generate-txs", "single-tx",
        "--source-seed", funding_seed,
        "--src-url", node_url,
        "--unshielded-amount", str(AMOUNT),
        "--unshielded-token-type", TOKEN_TYPE,
        "--destination-address", address,
        "--dest-url", node_url
    ]

    # Run the command (output is captured but we assume success if no error raised)
    run_command(cmd, cwd=cwd, verbose=verbose)

def process_chunk(target_indices, funding_seeds, node_url, verbose=False):
    failed_seeds = []
    try:
        relay_name = node_url.split('//')[1].split('.')[0]
    except IndexError:
        relay_name = "unknown"
    print(f"🚀 Starting chunk of {len(target_indices)} wallets on {relay_name}")

    with tempfile.TemporaryDirectory() as temp_dir:
        # Copy toolkit.db to temp_dir to avoid locking
        if os.path.exists(DB_PATH):
            shutil.copy(DB_PATH, os.path.join(temp_dir, "toolkit.db"))

        for i, seed in zip(target_indices, funding_seeds):
            try:
                print(f"[Chunk {seed[-4:]}] Generating wallet {i}...", end=" ", flush=True)
                addr = get_wallet_address(i, cwd=temp_dir, verbose=verbose)
                print(f"✅ {addr}")

                print(f"[Chunk {seed[-4:]}] Funding {addr}...", end=" ", flush=True)
                fund_address(addr, seed, node_url, cwd=temp_dir, verbose=verbose)
                print("✅ Sent")

                # Wait a bit between transactions to ensure nonce propagation
                time.sleep(2)
            except Exception as e:
                print(f"\n❌ Failed processing index {i}: {e}")
                failed_seeds.append(i)
    return failed_seeds

def main():
    if "MN_DONT_WATCH_PROGRESS" in os.environ:
        del os.environ["MN_DONT_WATCH_PROGRESS"]
    parser = argparse.ArgumentParser(description="Fund wallets.")
    parser.add_argument("--start", type=int, default=TARGET_START_INDEX, help="Starting seed to be funded")
    parser.add_argument("--end", type=int, default=TARGET_END_INDEX, help="Ending seed to be funded")
    parser.add_argument("--funding-start", type=int, default=FUNDING_START_INDEX, help="Starting funding seed index")
    parser.add_argument("--funding-end", type=int, default=FUNDING_END_INDEX, help="Ending funding seed index")
    parser.add_argument("--night-amount", type=float, default=FUNDING_AMOUNT, help="Amount of NIGHT tokens to fund")
    parser.add_argument("--verbose", action="store_true", help="Enable verbose output")
    parser.add_argument("--funding-indices", nargs='+', help="List of specific funding seed indices (space or comma-separated, overrides --funding-start/--funding-end)")
    parser.add_argument("--indices", nargs='+', help="List of specific seed indices to fund (space or comma-separated, overrides --start/--end)")
    parser.add_argument("--node-url", type=str, default=NODE_URL, help="Node URL. 'ferdie' will be replaced by relay names if present.")
    args = parser.parse_args()

    global AMOUNT
    AMOUNT = int(args.night_amount * 10**6)

    if args.indices:
        target_indices = []
        for item in args.indices:
            try:
                target_indices.extend([int(i.strip()) for i in item.split(',') if i.strip()])
            except ValueError:
                print(f"❌ Error: Invalid value in --indices: '{item}'. Please provide a list of integers.")
                sys.exit(1)
    else:
        target_indices = list(range(args.start, args.end + 1))

    if args.funding_indices:
        funding_indices = []
        for item in args.funding_indices:
            try:
                funding_indices.extend([int(i.strip()) for i in item.split(',') if i.strip()])
            except ValueError:
                print(f"❌ Error: Invalid value in --funding-indices: '{item}'. Please provide a list of integers.")
                sys.exit(1)
    elif FUNDING_SEEDS:
        funding_indices = FUNDING_SEEDS
    else:
        funding_indices = list(range(args.funding_start, args.funding_end + 1))

    source_seeds = [f"{i:064}" for i in funding_indices]

    print("🚀 Starting wallet creation and funding script...")

    total_wallets = len(target_indices)
    # Determine the number of workers based on the minimum of available resources
    cpu_count = os.cpu_count() or 1
    max_threads = max(1, int(cpu_count * 0.9))
    num_workers = min(len(source_seeds), max_threads)
    print(f"ℹ️  Using {num_workers} threads for execution.")

    if num_workers == 0:
        print("❌ No funding seeds or relays configured. Exiting.")
        sys.exit(1)
    chunk_size = math.ceil(total_wallets / num_workers)

    start_time = time.time()

    with concurrent.futures.ThreadPoolExecutor(max_workers=num_workers) as executor:
        futures = []
        for i in range(num_workers):
            chunk_indices = target_indices[i * chunk_size : (i + 1) * chunk_size]
            if not chunk_indices:
                break

            # Round-robin selection of relay node
            relay_name = RELAYS[i % len(RELAYS)]
            if "ferdie" in args.node_url:
                node_url = args.node_url.replace("ferdie", relay_name)
            else:
                node_url = args.node_url

            # Calculate which seeds belong to this chunk
            # We use modulo to cycle seeds if there are fewer seeds than wallets
            chunk_len = len(chunk_indices)
            chunk_seeds = [source_seeds[(i * chunk_size + k) % len(source_seeds)] for k in range(chunk_len)]

            futures.append(executor.submit(process_chunk, chunk_indices, chunk_seeds, node_url, args.verbose))

        failed_seeds = []
        for future in concurrent.futures.as_completed(futures):
            failed_seeds.extend(future.result())

    end_time = time.time()
    total_duration = end_time - start_time

    print("\n🎉 All operations completed successfully.")
    if total_duration > 120:
        minutes = int(total_duration // 60)
        seconds = total_duration % 60
        print(f"⏱️ Total execution time for {total_wallets} wallets: {minutes} minutes and {seconds:.2f} seconds")
    else:
        print(f"⏱️ Total execution time for {total_wallets} wallets: {total_duration:.2f} seconds")
    if total_wallets > 0:
        print(f"📊 Average time per funding: {total_duration / total_wallets:.2f} seconds")
    if failed_seeds:
        print(f"❌ Failed seeds: {sorted(failed_seeds)}")
        sys.exit(1)

if __name__ == "__main__":
    main()
