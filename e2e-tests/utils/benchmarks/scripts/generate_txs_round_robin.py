#!/usr/bin/env python3
import subprocess
import json
import sys
import argparse
import os
import time
import concurrent.futures
import random
import shutil
import tempfile

# Configuration
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
TOOLKIT_CMD = "midnight-node-toolkit"
TOKEN_TYPE = "0000000000000000000000000000000000000000000000000000000000000000"
BASE_AMOUNT = 1000000
START_INDEX = 1
END_INDEX = 500
DB_PATH = "toolkit.db"
NODE_URL = "ws://ferdie.node.sc.iog.io:9944" # "ws://localhost:9944"
DELAY = 0.25
MAX_RETRIES = 5


def run_command(cmd, cwd=None, verbose=False, exit_on_error=True):
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
        if not exit_on_error:
            raise e
        print(f"\n❌ Error executing command: {' '.join(cmd)}")
        print(f"STDOUT: {e.stdout}")
        print(f"STDERR: {e.stderr}")
        sys.exit(1)
    except FileNotFoundError:
        print(f"\n❌ Error: Executable '{cmd[0]}' not found. Ensure it is in your PATH.")
        sys.exit(1)

def get_address_for_seed_index(index, cwd=None, verbose=False):
    """Derives the address for a given seed index (padded to 64 chars)."""
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
    except (json.JSONDecodeError, KeyError):
        print(f"\n❌ Failed to parse address for seed index {index}")
        sys.exit(1)

def send_transaction(source_index, dest_address, amount_val, node_url_pattern, max_retries, save_to_file=True, cwd=None, verbose=False):
    """Sends a transaction from source seed index to destination address."""
    source_seed = f"{source_index:064}"
    amount = str(amount_val)

    start_relay_idx = source_index % len(RELAYS)
    num_attempts = min(len(RELAYS), max_retries)

    for i in range(num_attempts):
        relay_idx = (start_relay_idx + i) % len(RELAYS)
        relay_name = RELAYS[relay_idx]
        if relay_name.startswith("ws://") or relay_name.startswith("wss://"):
            node_url = relay_name
        elif "ferdie" in node_url_pattern:
            node_url = node_url_pattern.replace("ferdie", relay_name)
        else:
            node_url = node_url_pattern

        cmd = [
            TOOLKIT_CMD, "generate-txs", "single-tx",
            "--source-seed", source_seed,
            "--src-url", node_url,
            "--unshielded-amount", amount,
            "--unshielded-token-type", TOKEN_TYPE,
            "--destination-address", dest_address,
        ]

        if save_to_file:
            timestamp = int(time.time())
            filename = os.path.join("txs", f"tx_{timestamp}_{source_index}.mn")
            filename = os.path.abspath(filename)
            cmd.extend(["--dest-file", filename])
            cmd.extend(["--to-bytes"])
        else:
            cmd.extend(["--dest-url", node_url])

        try:
            last_attempt = (i == num_attempts - 1)
            run_command(cmd, cwd=cwd, verbose=verbose, exit_on_error=last_attempt)
            if i > 0:
                print(f"✅ [Seed {source_index}] Retry successful on {relay_name}")
            return
        except subprocess.CalledProcessError as e:
            # Stop retrying if the wallet has no funds or panic occurs
            if "There are no fundings" in e.stderr or "PanicError" in e.stderr:
                print(f"❌ [Seed {source_index}] Fatal error on {relay_name}: Insufficient funds or Panic.")
                raise e

            print(f"⚠️  [Seed {source_index}] Failed on {node_url}, trying next node...")
            time.sleep(0.5)

def process_transfer(i, start_index, end_index, node_url_pattern, max_retries, save_to_file, verbose, delay):
    """Handles the transfer for a single index in the ring."""
    try:
        # Calculate target index (circle back to start at the end)
        target_index = i + 1 if i < end_index else start_index

        # Randomize amount: BASE_AMOUNT +/- [1, 100]
        amount_val = BASE_AMOUNT + random.randint(-100, 100)
        print(f"Processing: Seed {i} -> Seed {target_index} (Amount: {amount_val})...")

        with tempfile.TemporaryDirectory() as temp_dir:
            # Copy toolkit.db to temp_dir to avoid locking
            db_copy_start = time.time()
            if os.path.exists(DB_PATH):
                shutil.copy(DB_PATH, os.path.join(temp_dir, "toolkit.db"))
            db_copy_time = time.time() - db_copy_start

            exec_start = time.time()
            dest_addr = get_address_for_seed_index(target_index, cwd=temp_dir, verbose=verbose)
            send_transaction(i, dest_addr, amount_val, node_url_pattern, max_retries, save_to_file=save_to_file, cwd=temp_dir, verbose=verbose)
            if delay > 0:
                time.sleep(delay)
            exec_time = time.time() - exec_start

        action = "Saved" if save_to_file else "Sent"
        print(f"✅ Seed {i} -> Seed {target_index} {action} ({amount_val}) [DB Copy: {db_copy_time:.4f}s, Exec: {exec_time:.4f}s]")
        return True
    except (SystemExit, Exception):
        return False

def main():
    parser = argparse.ArgumentParser(description="Generate or submit round-robin transactions.")
    parser.add_argument("--submit", action="store_true", help="Submit transactions directly instead of saving to file.")
    parser.add_argument("-v", "--verbose", action="store_true", help="Enable verbose output from toolkit commands.")
    parser.add_argument("-s", "--dest-start", type=int, default=START_INDEX, help="Starting seed to generate txs")
    parser.add_argument("-e", "--dest-end", type=int, default=END_INDEX, help="Ending seed to generate txs")
    parser.add_argument("--node-url", type=str, default=NODE_URL, help="Node URL. 'ferdie' will be replaced by relay names if present.")
    parser.add_argument("--delay", type=float, default=DELAY, help="Delay in seconds after each transaction generation.")
    parser.add_argument("--max-retries", type=int, default=MAX_RETRIES, help="Maximum number of attempts per transaction.")
    args = parser.parse_args()
    save_to_file = not args.submit
    verbose = args.verbose
    start_index = args.dest_start
    end_index = args.dest_end

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

    print(f"🔄 Fetching latest chain state from {args.node_url}...")
    run_command([TOOLKIT_CMD, "fetch", "-s", args.node_url], verbose=verbose)

    if save_to_file:
        if os.path.exists("txs"):
            shutil.rmtree("txs")
        os.makedirs("txs", exist_ok=True)

    start_time = time.time()
    print(f"🚀 Starting ring transaction script ({start_index} -> {start_index+1} -> ... -> {end_index} -> {start_index})...")

    num_txs = end_index - start_index + 1
    cpu_count = os.cpu_count() or 1
    max_threads = max(1, int(cpu_count * 0.9))
    max_workers = min(max_threads, num_txs)
    print(f"ℹ️  Using {max_workers} threads for execution.")
    results = []
    failed_seeds = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_to_seed = {executor.submit(process_transfer, i, start_index, end_index, args.node_url, args.max_retries, save_to_file, verbose, args.delay): i for i in range(start_index, end_index + 1)}
        for future in concurrent.futures.as_completed(future_to_seed):
            seed = future_to_seed[future]
            try:
                success = future.result()
                results.append(success)
                if not success:
                    failed_seeds.append(seed)
            except (SystemExit, Exception):
                results.append(False)
                failed_seeds.append(seed)

    end_time = time.time()
    print("\n🎉 Batch processing complete.")
    print(f"Valid: {results.count(True)}, Invalid: {results.count(False)}")
    if failed_seeds:
        print(f"❌ Failed {len(failed_seeds)} seeds: {sorted(failed_seeds)}")
    print(f"⏱️ Total execution time: {end_time - start_time:.2f} seconds")

if __name__ == "__main__":
    main()
