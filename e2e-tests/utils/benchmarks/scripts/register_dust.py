import subprocess
import time
import sys
import concurrent.futures
import math
import shutil
import tempfile
import os
import argparse
import random


# Configuration
TARGET_START_INDEX = 4
TARGET_END_INDEX = 499
FUNDING_START_INDEX = 1
FUNDING_END_INDEX = 3
FUNDING_SEEDS = []
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
TOOLKIT_PATH = "midnight-node-toolkit"
DB_PATH = "toolkit.db"
NODE_URL = "ws://ferdie.node.sc.iog.io:9944" # "ws://localhost:9944"
DELAY = 0.25


def register_chunk(indices, funding_seed, node_url, toolkit_path, verbose=False):
    failed_seeds = []
    try:
        relay_name = node_url.split('//')[1].split('.')[0]
    except IndexError:
        relay_name = "unknown"
    print(f"🚀 Starting chunk of {len(indices)} wallets on {relay_name} with funding seed ...{funding_seed[-2:]}")

    with tempfile.TemporaryDirectory() as temp_dir:
        if os.path.exists(DB_PATH):
            shutil.copy(DB_PATH, os.path.join(temp_dir, "toolkit.db"))

        for i in indices:
            # Format the seed: Pad '20' to '000...00020' (64 chars total)
            wallet_seed = f"{i:064}"

            time.sleep(random.uniform(DELAY * 0.5, DELAY * 1.5))

            print(f"[Chunk {funding_seed[-2:]}] Registering dust for seed ...{i}...")

            cmd = [
                toolkit_path, "generate-txs",
                "--src-url", node_url,
                "--dest-url", node_url,
                "register-dust-address",
                "--wallet-seed", wallet_seed,
                "--funding-seed", funding_seed
            ]

            if verbose:
                print(f"CMD: {' '.join(cmd)}")

            try:
                result = subprocess.run(
                    cmd,
                    capture_output=True,
                    text=True,
                    check=True,
                    cwd=temp_dir
                )
                print(f"✅ Success (Seed ...{i})")
                if verbose:
                    print(f"STDOUT:\n{result.stdout}")
                    print(f"STDERR:\n{result.stderr}")

            except subprocess.CalledProcessError as e:
                print(f"\n❌ Failed to register seed ...{i}!")
                if verbose:
                    print(f"STDOUT:\n{e.stdout}")
                    print(f"STDERR:\n{e.stderr}")
                if "len (is 0)" in e.stderr:
                    print("💡 Hint: The funding wallet likely has no funds (0 UTXOs).")
                print("Error Output:", e.stderr)
                # We continue to the next one even if one fails
                failed_seeds.append(i)

            except FileNotFoundError:
                print(f"\n❌ Error: Could not find '{toolkit_path}'.")
                sys.exit(1)
    return failed_seeds


def register_dust_addresses():
    if "MN_DONT_WATCH_PROGRESS" in os.environ:
        del os.environ["MN_DONT_WATCH_PROGRESS"]
    parser = argparse.ArgumentParser(description="Register dust addresses.")
    parser.add_argument("--dest-start", type=int, default=TARGET_START_INDEX, help="Starting seed to be registered")
    parser.add_argument("--dest-end", type=int, default=TARGET_END_INDEX, help="Ending seed to be registered")
    parser.add_argument("--fund-start", type=int, default=FUNDING_START_INDEX, help="Starting funding seed index")
    parser.add_argument("--fund-end", type=int, default=FUNDING_END_INDEX, help="Ending funding seed index")
    parser.add_argument("--verbose", action="store_true", help="Enable verbose output")
    parser.add_argument("--fund-indices", nargs='+', help="List of specific funding seed indices (space or comma-separated, overrides --fund-start/--fund-end)")
    parser.add_argument("--dest-indices", nargs='+', help="List of specific seed indices to register (space or comma-separated, overrides --dest-start/--dest-end)")
    parser.add_argument("--node-url", type=str, default=NODE_URL, help="Node URL. 'ferdie' will be replaced by other relay names if present.")
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

    funding_seeds = [f"{i:064}" for i in funding_indices]

    print(f"🚀 Starting dust registration for {len(target_indices)} seeds...")

    total_wallets = len(target_indices)
    # Determine the number of workers based on the minimum of available resources
    cpu_count = os.cpu_count() or 1
    max_threads = max(1, int(cpu_count * 0.9))
    num_workers = min(total_wallets, len(funding_seeds), max_threads)
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
            futures.append(executor.submit(register_chunk, chunk_indices, funding_seeds[i], node_url, TOOLKIT_PATH, args.verbose))

        failed_seeds = []
        for future in concurrent.futures.as_completed(futures):
            failed_seeds.extend(future.result())

    end_time = time.time()
    total_duration = end_time - start_time

    print("\n🎉 All registration commands completed.")
    if total_duration > 120:
        minutes = int(total_duration // 60)
        seconds = total_duration % 60
        print(f"⏱️ Total execution time for {total_wallets} wallets: {minutes} minutes and {seconds:.2f} seconds")
    else:
        print(f"⏱️ Total execution time for {total_wallets} wallets: {total_duration:.2f} seconds")
    if total_wallets > 0:
        print(f"📊 Average time per registration: {total_duration / total_wallets:.2f} seconds")
    if failed_seeds:
        print(f"❌ Failed seeds: {sorted(failed_seeds)}")
        sys.exit(1)

if __name__ == "__main__":
    register_dust_addresses()