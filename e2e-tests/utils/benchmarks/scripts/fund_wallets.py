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
TARGET_START_INDEX = 4
TARGET_END_INDEX = 10
FUNDING_START_INDEX = 1
FUNDING_END_INDEX = 3
TOKEN_TYPE = "0000000000000000000000000000000000000000000000000000000000000000"
DB_PATH = "toolkit.db"
NODE_URL = "ws://ferdie.node.sc.iog.io:9944" # "ws://localhost:9944"
FUNDING_AMOUNT = 3000000
FUNDING_SEEDS = []
MAX_RETRIES = 10
DELAY = 0.25

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

        # Check for RPC errors that might not cause a non-zero exit code
        if "RPC error" in result.stdout or "RPC error" in result.stderr:
            raise subprocess.CalledProcessError(result.returncode, cmd, output=result.stdout, stderr=result.stderr)

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
                print(f"[Chunk {seed[-4:]}] Generating wallet {i}...")
                addr = get_wallet_address(i, cwd=temp_dir, verbose=verbose)
                print(f"✅ Wallet {i}: {addr}")

                time.sleep(random.uniform(DELAY * 0.5, DELAY * 1.5))

                print(f"[Chunk {seed[-4:]}] Funding {addr}...")
                for attempt in range(MAX_RETRIES):
                    try:
                        fund_address(addr, seed, node_url, cwd=temp_dir, verbose=verbose)
                        print(f"✅ Funding sent to {addr}")
                        break
                    except subprocess.CalledProcessError:
                        if attempt < MAX_RETRIES - 1:
                            msg = f"⚠️  Retry {attempt+1}/{MAX_RETRIES} for {addr}..."

                            # Rotate relay node if possible
                            for r in RELAYS:
                                if r in node_url:
                                    next_r = RELAYS[(RELAYS.index(r) + 1) % len(RELAYS)]
                                    node_url = node_url.replace(r, next_r)
                                    msg += f" [Switched to {next_r}]"
                                    break
                            print(msg)
                            time.sleep(random.uniform(2, 5) + (attempt * 2))
                        else:
                            raise

                # Wait a bit between transactions to ensure nonce propagation
                time.sleep(2)
            except Exception as e:
                print(f"\n❌ Failed processing index {i}: {e}")
                failed_seeds.append(i)
    return failed_seeds

def check_night_balances(funding_indices, amount_per_wallet, total_wallets, node_url):
    """
    Checks if funding seeds have sufficient balance and returns a list of valid seeds.
    """
    num_seeds = len(funding_indices)
    if num_seeds == 0: return []

    wallets_per_seed = math.ceil(total_wallets / num_seeds)
    required_star = (wallets_per_seed * amount_per_wallet) + (1 * amount_per_wallet)

    print(f"🔍 Checking balances for {num_seeds} funding seeds...")
    print(f"   Est. required per seed: {required_star/1_000_000:.2f} NIGHT (includes 1x buffer)")

    script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "get_night_balances.py")
    if not os.path.exists(script_path):
        print(f"⚠️  Warning: {script_path} not found. Skipping balance check.")
        return funding_indices

    # We check all funding indices at once
    indices_str = ",".join(map(str, funding_indices))
    cmd = [sys.executable, script_path, "--dest-indices", indices_str, "--node-url", node_url]

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)

        insufficient_seeds = {} # Using a dict to store balance info
        for line in result.stdout.splitlines():
            if line.strip().startswith("Seed "):
                parts = line.split()
                try:
                    seed_idx = int(parts[1].rstrip(':'))
                    balance = int(parts[2])
                    if balance < required_star:
                        insufficient_seeds[seed_idx] = balance
                except (ValueError, IndexError):
                    continue

        valid_indices = [idx for idx in funding_indices if idx not in insufficient_seeds]

        if insufficient_seeds:
            print("⚠️  Insufficient night funds detected in some seeds. They will be excluded from funding.")
            for seed_idx, balance in sorted(insufficient_seeds.items()):
                print(f"   - Seed {seed_idx}: Has {balance/1_000_000:.2f} NIGHT, needs {required_star/1_000_000:.2f} NIGHT")

        if len(valid_indices) == len(funding_indices):
             print("✅ All funding seeds have sufficient night balances.")

        return valid_indices

    except subprocess.CalledProcessError:
        print("❌ Error running get_night_balances.py. Cannot verify balances.")
        return []

def check_dust_balances(funding_indices, total_wallets, node_url):
    """
    Checks if funding seeds have sufficient dust balance and returns a list of valid seeds.
    """
    num_seeds = len(funding_indices)
    if num_seeds == 0: return []

    # Assuming 1 dust per transaction + buffer
    DUST_PER_TX = 1
    wallets_per_seed = math.ceil(total_wallets / num_seeds)
    required_dust = (wallets_per_seed * DUST_PER_TX) + (2 * DUST_PER_TX)

    print(f"🔍 Checking dust balances for {num_seeds} funding seeds...")
    print(f"   Est. required per seed: {required_dust} DUST")

    script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "get_dust_balances.py")
    if not os.path.exists(script_path):
        print(f"⚠️  Warning: {script_path} not found. Skipping balance check.")
        return funding_indices

    # We check all funding indices at once
    indices_str = ",".join(map(str, funding_indices))
    cmd = [sys.executable, script_path, "--dest-indices", indices_str, "--node-url", node_url]

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)

        insufficient_seeds = {}
        for line in result.stdout.splitlines():
            if line.strip().startswith("Seed "):
                parts = line.split()
                try:
                    seed_idx = int(parts[1].rstrip(':'))
                    balance = int(parts[2])
                    if balance < required_dust:
                        insufficient_seeds[seed_idx] = balance
                except (ValueError, IndexError):
                    continue

        valid_indices = [idx for idx in funding_indices if idx not in insufficient_seeds]

        if insufficient_seeds:
            print("⚠️  Insufficient dust detected in some seeds. They will be excluded from funding.")
            for seed_idx, balance in sorted(insufficient_seeds.items()):
                print(f"   - Seed {seed_idx}: Has {balance} DUST, needs {required_dust} DUST")

        if len(valid_indices) == len(funding_indices):
             print("✅ All funding seeds have sufficient dust balances.")

        return valid_indices

    except subprocess.CalledProcessError:
        print("❌ Error running get_dust_balances.py. Cannot verify balances.")
        return []

def format_indices_string(indices):
    """Returns a string representation of indices (range if consecutive, list otherwise)."""
    if not indices:
        return "None"

    sorted_indices = sorted(indices)
    is_consecutive = (sorted_indices[-1] - sorted_indices[0] == len(sorted_indices) - 1)

    if is_consecutive and len(indices) > 1:
        return f"{sorted_indices[0]}-{sorted_indices[-1]}"
    else:
        return ", ".join(map(str, sorted_indices))

def main():
    if "MN_DONT_WATCH_PROGRESS" in os.environ:
        del os.environ["MN_DONT_WATCH_PROGRESS"]
    parser = argparse.ArgumentParser(description="Fund wallets.")
    parser.add_argument("-s", "--dest-start", type=int, default=TARGET_START_INDEX, help="Starting seed to be funded")
    parser.add_argument("-e", "--dest-end", type=int, default=TARGET_END_INDEX, help="Ending seed to be funded")
    parser.add_argument("--fund-start", type=int, default=FUNDING_START_INDEX, help="Starting funding seed index")
    parser.add_argument("--fund-end", type=int, default=FUNDING_END_INDEX, help="Ending funding seed index")
    parser.add_argument("-a", "--night-amount", type=float, default=FUNDING_AMOUNT, help="Amount of NIGHT tokens to fund")
    parser.add_argument("-v", "--verbose", action="store_true", help="Enable verbose output")
    parser.add_argument("--fund-indices", nargs='+', help="List of specific funding seed indices (space or comma-separated, overrides --fund-start/--fund-end)")
    parser.add_argument("-i", "--dest-indices", nargs='+', help="List of specific seed indices to fund (space or comma-separated, overrides --dest-start/--dest-end)")
    parser.add_argument("--node-url", type=str, default=NODE_URL, help="Node URL. 'ferdie' will be replaced by relay names if present.")
    parser.add_argument("--check-balances", action="store_true", help="Perform balance checks (default: False)")
    args = parser.parse_args()

    global AMOUNT
    AMOUNT = int(args.night_amount * 10**6)

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

    if args.fund_indices:
        funding_indices = []
        for item in args.fund_indices:
            try:
                funding_indices.extend([int(i.strip()) for i in item.split(',') if i.strip()])
            except ValueError:
                print(f"❌ Error: Invalid value in --fund-indices: '{item}'. Please provide a list of integers.")
                sys.exit(1)
    elif FUNDING_SEEDS:
        funding_indices = FUNDING_SEEDS
    else:
        funding_indices = list(range(args.fund_start, args.fund_end + 1))

    # Check balances before proceeding
    if args.check_balances:
        original_seed_count = len(funding_indices)
        funding_indices = check_night_balances(funding_indices, AMOUNT, len(target_indices), args.node_url)

        if not funding_indices:
            print("❌ No funding seeds with sufficient balance available. Aborting.")
            sys.exit(1)

        funding_indices = check_dust_balances(funding_indices, len(target_indices), args.node_url)
        if not funding_indices:
            print("❌ No funding seeds with sufficient dust balance available. Aborting.")
            sys.exit(1)

        if len(funding_indices) < original_seed_count:
            print(f"ℹ️  Continuing with {len(funding_indices)} of {original_seed_count} funding seeds.")

    source_seeds = [f"{i:064}" for i in funding_indices]

    print(f"🚀 Starting wallet creation and funding script for seeds {format_indices_string(target_indices)}...")

    total_wallets = len(target_indices)
    # Determine the number of workers based on the minimum of available resources
    cpu_count = os.cpu_count() or 1
    max_threads = max(1, int(cpu_count * 0.5))
    num_workers = min(len(source_seeds), max_threads)
    print(f"ℹ️  Using {num_workers} threads for execution.")

    if num_workers == 0:
        if total_wallets == 0:
            print("ℹ️  No wallets to fund.")
            sys.exit(0)
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
            if relay_name.startswith("ws://") or relay_name.startswith("wss://"):
                node_url = relay_name
            elif "ferdie" in args.node_url:
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
