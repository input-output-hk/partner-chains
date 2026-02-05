import subprocess
import glob
import os
import re
import sys
import time
import concurrent.futures
import tempfile
import json
import argparse
import random

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
NODE_URL = "ws://ferdie.node.sc.iog.io:9944" # "ws://localhost:9944"
MAX_RETRIES = 5
DELAY = 0.25

def submit_single_tx(i, tx_file, total_files, toolkit_path, node_url_pattern, max_retries, verbose=False, max_workers=1, delay=DELAY):
    # Ensure absolute path for the source file since we change CWD
    abs_tx_file = os.path.abspath(tx_file)

    start_relay_idx = i % len(RELAYS)
    num_attempts = min(len(RELAYS), max_retries)

    for r_offset in range(num_attempts):
        relay_idx = (start_relay_idx + r_offset) % len(RELAYS)
        relay_name = RELAYS[relay_idx]
        if relay_name.startswith("ws://") or relay_name.startswith("wss://"):
            dest_url = relay_name
        elif "ferdie" in node_url_pattern:
            dest_url = node_url_pattern.replace("ferdie", relay_name)
        else:
            dest_url = node_url_pattern

        cmd = [
            toolkit_path, "generate-txs", "send",
            "--src-file", abs_tx_file,
            "--dest-url", dest_url
        ]

        if verbose:
            print(f"[{i}/{total_files}] Running: {' '.join(cmd)}")

        try:
            with tempfile.TemporaryDirectory() as temp_dir:
                exec_start = time.time()
                result = subprocess.run(
                    cmd, capture_output=True, text=True, check=True, cwd=temp_dir
                )
                if verbose:
                    if result.stdout:
                        print(f"[{i}/{total_files}] STDOUT: {result.stdout.strip()}")
                    if result.stderr:
                        print(f"[{i}/{total_files}] STDERR: {result.stderr.strip()}")
                exec_time = time.time() - exec_start

            chain_latency = 0.0
            sent_ts = 0
            finalized_ts = 0

            if result.stderr:
                print(f"⚠️  {tx_file}: {result.stderr}")
                try:
                    for line in result.stderr.splitlines():
                        if '"message":"SENT"' in line:
                            sent_ts = json.loads(line[line.find("{"):])["timestamp"]
                        elif '"message":"FINALIZED"' in line:
                            finalized_ts = json.loads(line[line.find("{"):])["timestamp"]
                    if sent_ts and finalized_ts:
                        chain_latency = (finalized_ts - sent_ts) / 1000.0
                except Exception:
                    pass

            if "RPC error: User error: Invalid Transaction (1010)" in result.stdout or \
               "RPC error: User error: Invalid Transaction (1010)" in result.stderr:
                print(f"❌ [{i}/{total_files}] Failed to send {tx_file} to {relay_name} (Invalid Transaction 1010) [Exec: {exec_time:.4f}s]")
                return False

            if "RPC error: User error: Transaction is temporarily banned (1012)" in result.stdout or \
               "RPC error: User error: Transaction is temporarily banned (1012)" in result.stderr:
                print(f"⛔ [{i}/{total_files}] Failed to send {tx_file} to {relay_name} (Temporarily Banned 1012) [Exec: {exec_time:.4f}s]")
                return "Banned"

            print(f"✅ [{i}/{total_files}] Sent {tx_file} to {relay_name} [Chain Latency: {chain_latency:.2f}s, Exec: {exec_time:.4f}s]")
            if delay > 0:
                time.sleep(random.uniform(delay * 0.5, delay * 1.5))
            return True

        except subprocess.CalledProcessError as e:
            if r_offset == num_attempts - 1:
                print(f"\n❌ Failed to submit {tx_file} to {relay_name}!")
                print("Error Output:", e.stderr)
                return False
            else:
                print(f"⚠️  Failed to submit {tx_file} to {dest_url}, trying next node...")
                time.sleep(0.5)

def submit_transactions(toolkit_path="midnight-node-toolkit"):
    # Disable watching for txs to finalize
    os.environ["MN_DONT_WATCH_PROGRESS"] = "true"

    parser = argparse.ArgumentParser(description="Submit batch transactions.")
    parser.add_argument("-s", "--dest-start", type=int, help="Start index")
    parser.add_argument("-e", "--dest-end", type=int, help="End index")
    parser.add_argument("-v", "--verbose", action="store_true", help="Enable verbose output")
    parser.add_argument("--workers", type=int, help="Number of concurrent workers")
    parser.add_argument("--node-url", type=str, default=NODE_URL, help="Node URL. 'ferdie' will be replaced by relay names if present.")
    parser.add_argument("--max-retries", type=int, default=MAX_RETRIES, help="Maximum number of attempts per transaction.")
    parser.add_argument("--delay", type=float, default=DELAY, help="Delay in seconds after each transaction submission.")
    parser.add_argument("--batch-size", type=int, default=0, help="Number of transactions to submit per batch. Default: 0 (submit all at once).")
    parser.add_argument("--batch-delay", type=float, default=6.0, help="Delay in seconds between batches.")
    parser.add_argument("--tx-dir", type=str, default="txs", help="Directory containing transaction files.")
    args = parser.parse_args()

    start_time = time.time()
    # 1. Find all matching files
    all_files = glob.glob(os.path.join(args.tx_dir, "tx_*.mn"))
    all_files.sort()

    files = []
    if args.dest_start is None and args.dest_end is None:
        files = all_files
    else:
        for f in all_files:
            try:
                basename = os.path.basename(f)
                index = int(os.path.splitext(basename)[0].split('_')[-1])
                if args.dest_start is not None and index < args.dest_start:
                    continue
                if args.dest_end is not None and index > args.dest_end:
                    continue
                files.append(f)
            except (ValueError, IndexError):
                continue

    if not files:
        msg = f" in range {args.dest_start}-{args.dest_end}" if args.dest_start is not None else ""
        print(f"❌ No files found matching 'tx_*.mn'{msg}")
        sys.exit(1)

    print(f"🚀 Found {len(files)} transaction files to submit.")

    if args.batch_size > 0:
        print(f"📦 Batching enabled: {args.batch_size} txs/batch, {args.batch_delay}s delay between batches.")
    else:
        print(f"📦 Batching disabled: Submitting all transactions in a single batch.")

    if args.workers:
        max_workers = args.workers
    else:
        cpu_count = os.cpu_count() or 1
        max_threads = max(1, int(cpu_count * 0.9))
        max_workers = min(max_threads, len(files))
    print(f"ℹ️  Using {max_workers} threads for execution.")

    results = []
    failed_seeds = []

    # Determine batches
    if args.batch_size and args.batch_size > 0:
        batches = [files[i:i + args.batch_size] for i in range(0, len(files), args.batch_size)]
    else:
        batches = [files]

    total_files_count = len(files)
    global_index = 1

    for batch_idx, batch in enumerate(batches):
        if batch_idx > 0:
            print(f"⏳ Waiting {args.batch_delay}s before next batch...")
            time.sleep(args.batch_delay)

        if args.batch_size > 0:
            print(f"🚀 Processing Batch {batch_idx + 1}/{len(batches)}: {len(batch)} transactions")

        with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
            future_to_file = {executor.submit(submit_single_tx, global_index + i, tx_file, total_files_count, toolkit_path, args.node_url, args.max_retries, verbose=args.verbose, max_workers=max_workers, delay=args.delay): tx_file for i, tx_file in enumerate(batch)}
            for future in concurrent.futures.as_completed(future_to_file):
                tx_file = future_to_file[future]
                try:
                    res = future.result()
                    results.append(res)
                    if res is False:
                        basename = os.path.basename(tx_file)
                        seed = os.path.splitext(basename)[0].split('_')[-1]
                        failed_seeds.append(seed)
                except Exception:
                    results.append(False)
                    basename = os.path.basename(tx_file)
                    seed = os.path.splitext(basename)[0].split('_')[-1]
                    failed_seeds.append(seed)

        global_index += len(batch)

    end_time = time.time()
    print("\n🎉 Batch submission complete.")
    print(f"Valid: {results.count(True)}, Invalid: {results.count(False)}, Temporarily Banned: {results.count('Banned')}")
    if failed_seeds:
        try:
            failed_seeds.sort(key=int)
        except ValueError:
            failed_seeds.sort()
        print(f"❌ Failed seeds: {failed_seeds}")
    print(f"⏱️ Total execution time: {end_time - start_time:.2f} seconds")

if __name__ == "__main__":
    submit_transactions()
