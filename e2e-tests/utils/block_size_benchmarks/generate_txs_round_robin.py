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
TOOLKIT_CMD = "midnight-node-toolkit"
TOKEN_TYPE = "0000000000000000000000000000000000000000000000000000000000000000"
BASE_AMOUNT = 1000000
START_INDEX = 10
END_INDEX = 499
DB_PATH = "toolkit.db"

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

def send_transaction(source_index, dest_address, amount_val, save_to_file=True, cwd=None, verbose=False):
    """Sends a transaction from source seed index to destination address."""
    source_seed = f"{source_index:064}"
    amount = str(amount_val)
    
    start_relay_idx = source_index % len(RELAYS)

    for i in range(len(RELAYS)):
        relay_idx = (start_relay_idx + i) % len(RELAYS)
        relay_name = RELAYS[relay_idx]
        node_url = f"ws://{relay_name}.node.sc.iog.io:9944"

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
            last_attempt = (i == len(RELAYS) - 1)
            run_command(cmd, cwd=cwd, verbose=verbose, exit_on_error=last_attempt)
            if i > 0:
                print(f"✅ Retry successful on {relay_name}")
            return
        except subprocess.CalledProcessError:
            print(f"⚠️  Failed on {relay_name}, trying next node...")

def process_transfer(i, start_index, end_index, save_to_file, verbose):
    """Handles the transfer for a single index in the ring."""
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
        send_transaction(i, dest_addr, amount_val, save_to_file=save_to_file, cwd=temp_dir, verbose=verbose)
        exec_time = time.time() - exec_start
    
    action = "Saved" if save_to_file else "Sent"
    print(f"✅ Seed {i} -> Seed {target_index} {action} ({amount_val}) [DB Copy: {db_copy_time:.4f}s, Exec: {exec_time:.4f}s]")

def main():
    parser = argparse.ArgumentParser(description="Generate or submit round-robin transactions.")
    parser.add_argument("--submit", action="store_true", help="Submit transactions directly instead of saving to file.")
    parser.add_argument("--verbose", action="store_true", help="Enable verbose output from toolkit commands.")
    parser.add_argument("--start", type=int, default=START_INDEX, help="Starting seed to generate txs")
    parser.add_argument("--end", type=int, default=END_INDEX, help="Ending seed to generate txs")
    args = parser.parse_args()
    save_to_file = not args.submit
    verbose = args.verbose
    start_index = args.start
    end_index = args.end

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

    if save_to_file:
        if os.path.exists("txs"):
            shutil.rmtree("txs")
        os.makedirs("txs", exist_ok=True)

    start_time = time.time()
    print(f"🚀 Starting ring transaction script ({start_index} -> {start_index+1} -> ... -> {end_index} -> {start_index})...")
   
    num_txs = end_index - start_index + 1
    max_workers = min(os.cpu_count() or 1, num_txs)
    print(f"ℹ️  Using {max_workers} threads for execution.")
    with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures = [executor.submit(process_transfer, i, start_index, end_index, save_to_file, verbose) for i in range(start_index, end_index + 1)]
        concurrent.futures.wait(futures)

    end_time = time.time()
    print("\n🎉 All transactions generated or sent successfully.")
    print(f"⏱️ Total execution time: {end_time - start_time:.2f} seconds")

if __name__ == "__main__":
    main()
