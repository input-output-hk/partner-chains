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

def submit_single_tx(i, tx_file, total_files, toolkit_path, verbose=False, max_workers=1):
    # Ensure absolute path for the source file since we change CWD
    abs_tx_file = os.path.abspath(tx_file)

    start_relay_idx = i % len(RELAYS)

    for r_offset in range(len(RELAYS)):
        relay_idx = (start_relay_idx + r_offset) % len(RELAYS)
        relay_name = RELAYS[relay_idx]
        dest_url = f"ws://{relay_name}.node.sc.iog.io:9944"

        cmd = [
            toolkit_path, "generate-txs", "send",
            "--src-file", abs_tx_file,
            "--fetch-cache", "inmemory",
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

            print(f"✅ [{i}/{total_files}] Sent {tx_file} to {relay_name} [Chain Latency: {chain_latency:.2f}s, Exec: {exec_time:.4f}s]")
            if max_workers > 1:
                time.sleep(random.uniform(0.05, 0.5))
            return

        except subprocess.CalledProcessError as e:
            if r_offset == len(RELAYS) - 1:
                print(f"\n❌ Failed to submit {tx_file} to {relay_name}!")
                print("Error Output:", e.stderr)
            else:
                print(f"⚠️  Failed to submit {tx_file} to {relay_name}, trying next node...")

def submit_transactions(toolkit_path="midnight-node-toolkit"):
    # Disable progress watching for all subprocesses
    os.environ["MN_DONT_WATCH_PROGRESS"] = "true"

    parser = argparse.ArgumentParser(description="Submit batch transactions.")
    parser.add_argument("--start", type=int, help="Start index")
    parser.add_argument("--end", type=int, help="End index")
    parser.add_argument("--verbose", action="store_true", help="Enable verbose output")
    parser.add_argument("--workers", type=int, help="Number of concurrent workers")
    args = parser.parse_args()

    start_time = time.time()
    # 1. Find all matching files
    all_files = glob.glob(os.path.join("txs", "tx_*.mn"))
    
    files = []
    if args.start is not None and args.end is not None:
        for f in all_files:
            try:
                basename = os.path.basename(f)
                index = int(os.path.splitext(basename)[0].split('_')[-1])
                if args.start <= index <= args.end:
                    files.append(f)
            except (ValueError, IndexError):
                continue
    else:
        files = all_files

    if not files:
        msg = f" in range {args.start}-{args.end}" if args.start is not None else ""
        print(f"❌ No files found matching 'tx_*.mn'{msg}")
        sys.exit(1)

    print(f"🚀 Found {len(files)} transaction files to submit.")

    if args.workers:
        max_workers = args.workers
    else:
        max_workers = min(os.cpu_count() or 1, len(files))
    print(f"ℹ️  Using {max_workers} threads for execution.")
    with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures = [executor.submit(submit_single_tx, i, tx_file, len(files), toolkit_path, verbose=args.verbose, max_workers=max_workers) for i, tx_file in enumerate(files, 1)]
        concurrent.futures.wait(futures)

    end_time = time.time()
    print("\n🎉 Batch submission complete.")
    print(f"⏱️ Total execution time: {end_time - start_time:.2f} seconds")

if __name__ == "__main__":
    submit_transactions()