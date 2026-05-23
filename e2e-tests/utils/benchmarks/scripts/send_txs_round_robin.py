#!/usr/bin/env python3
import subprocess
import json
import sys
import os
import time
import concurrent.futures
import random
import tempfile
import shutil

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
START_INDEX = 20
END_INDEX = 35
DB_PATH = "toolkit.db"

def run_command(cmd, cwd=None):
    """Runs a command and returns stdout if successful, exits otherwise."""
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True, cwd=cwd)
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        print(f"\n❌ Error executing command: {' '.join(cmd)}")
        print(f"STDOUT: {e.stdout}")
        print(f"STDERR: {e.stderr}")
        sys.exit(1)
    except FileNotFoundError:
        print(f"\n❌ Error: Executable '{cmd[0]}' not found. Ensure it is in your PATH.")
        sys.exit(1)

def get_address_for_seed_index(index, cwd=None):
    """Derives the address for a given seed index (padded to 64 chars)."""
    seed = f"{index:064}"
    
    cmd = [
        TOOLKIT_CMD, "show-address",
        "--network", "undeployed",
        "--seed", seed
    ]
    
    output = run_command(cmd, cwd=cwd)
    try:
        data = json.loads(output)
        return data["unshielded"]
    except (json.JSONDecodeError, KeyError):
        print(f"\n❌ Failed to parse address for seed index {index}")
        sys.exit(1)

def send_transaction(source_index, dest_address, amount_val, cwd=None):
    """Sends a transaction from source seed index to destination address."""
    source_seed = f"{source_index:064}"
    amount = str(amount_val)
    
    # Round-robin selection of relay node
    relay_name = RELAYS[source_index % len(RELAYS)]
    if relay_name.startswith("ws://") or relay_name.startswith("wss://"):
        node_url = relay_name
    else:
        node_url = f"ws://{relay_name}.node.sc.iog.io:9944"
    
    cmd = [
        TOOLKIT_CMD, "generate-txs", "single-tx",
        "--source-seed", source_seed,
        "--src-url", node_url,
        "--unshielded-amount", amount,
        "--unshielded-token-type", TOKEN_TYPE,
        "--destination-address", dest_address,
        "--dest-url", node_url
    ]
    
    run_command(cmd, cwd=cwd)

def process_transfer(i, start_index, end_index):
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
        dest_addr = get_address_for_seed_index(target_index, cwd=temp_dir)
        send_transaction(i, dest_addr, amount_val, cwd=temp_dir)
        exec_time = time.time() - exec_start
    
    print(f"✅ Seed {i} -> Seed {target_index} Sent ({amount_val}) [DB Copy: {db_copy_time:.4f}s, Exec: {exec_time:.4f}s]")

def main():
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
    print(f"🚀 Starting ring transaction script ({START_INDEX} -> {START_INDEX+1} -> ... -> {END_INDEX} -> {START_INDEX})...")
   
    num_txs = END_INDEX - START_INDEX + 1
    cpu_count = os.cpu_count() or 1
    max_threads = max(1, int(cpu_count * 0.9))
    max_workers = min(max_threads, num_txs)
    print(f"ℹ️  Using {max_workers} threads for execution.")
    with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures = [executor.submit(process_transfer, i, START_INDEX, END_INDEX) for i in range(START_INDEX, END_INDEX + 1)]
        concurrent.futures.wait(futures)

    end_time = time.time()
    print("\n🎉 All transactions sent successfully.")
    print(f"⏱️ Total execution time: {end_time - start_time:.2f} seconds")

if __name__ == "__main__":
    main()