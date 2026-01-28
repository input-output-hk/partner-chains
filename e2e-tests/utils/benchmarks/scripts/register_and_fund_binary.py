#!/usr/bin/env python3
import argparse
import subprocess
import sys
import os
import time
import logging

# Configuration
TOOLKIT_CMD = "midnight-node-toolkit"
NODE_URL = "ws://ferdie.node.sc.iog.io:9944" # "ws://localhost:9944"
NIGHT_AMOUNT = 1000000

def setup_logging(logfile=None):
    """Configures logging to a file if specified, otherwise suppresses logs."""
    log_format = "%(asctime)s - %(levelname)s - %(message)s"
    logger = logging.getLogger()
    logger.setLevel(logging.INFO)

    if logger.hasHandlers():
        logger.handlers.clear()

    # File handler (captures everything including verbose output)
    if logfile:
        file_handler = logging.FileHandler(logfile, mode='w')
        file_handler.setFormatter(logging.Formatter(log_format))
        logger.addHandler(file_handler)
    else:
        # Prevent printing to stderr if no logfile is specified
        logger.addHandler(logging.NullHandler())

def log_msg(message, level=logging.INFO, to_console=True):
    """Logs a message to the file and optionally prints it to the console."""
    if to_console:
        print(message)
    logging.log(level, message)

def run_batch_actions(fund_start, fund_end, dest_start, dest_end, amount, node_url):
    """Runs dust registration and then funds the wallets for a given batch."""
    # --- Register Dust ---
    log_msg(f"-> Step 1: Registering dust for wallets {dest_start}-{dest_end}")
    register_script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "register_dust.py")
    register_cmd = [
        sys.executable, register_script_path,
        "--fund-start", str(fund_start),
        "--fund-end", str(fund_end),
        "--dest-start", str(dest_start),
        "--dest-end", str(dest_end),
        "--node-url", node_url,
        "--verbose"
    ]
    log_msg(f"   Running: {' '.join(register_cmd)}")
    try:
        result = subprocess.run(register_cmd, check=True, capture_output=True, text=True)
        # Only log verbose output to file, not console
        if result.stdout: log_msg(f"   STDOUT from register_dust.py:\n{result.stdout.strip()}", to_console=False)
        if result.stderr: log_msg(f"   STDERR from register_dust.py:\n{result.stderr.strip()}", level=logging.WARNING, to_console=False)
    except subprocess.CalledProcessError as e:
        log_msg("   ❌ Error executing register_dust.py", level=logging.ERROR)
        if e.stdout: log_msg(f"   STDOUT: {e.stdout.strip()}", level=logging.ERROR, to_console=False)
        if e.stderr: log_msg(f"   STDERR: {e.stderr.strip()}", level=logging.ERROR, to_console=False)
        return False

    log_msg("   ✅ Dust registration complete for this batch.")
    time.sleep(1) # Small delay between steps

    # --- Fund Wallets ---
    log_msg(f"-> Step 2: Funding wallets {dest_start}-{dest_end} with {amount:.2f} NIGHT")
    fund_script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "fund_wallets.py")
    fund_cmd = [
        sys.executable, fund_script_path,
        "--fund-start", str(fund_start),
        "--fund-end", str(fund_end),
        "--dest-start", str(dest_start),
        "--dest-end", str(dest_end),
        "--night-amount", str(amount),
        "--node-url", node_url,
        "--verbose"
    ]
    log_msg(f"   Running: {' '.join(fund_cmd)}")
    try:
        result = subprocess.run(fund_cmd, check=True, capture_output=True, text=True)
        # Only log verbose output to file, not console
        if result.stdout: log_msg(f"   STDOUT from fund_wallets.py:\n{result.stdout.strip()}", to_console=False)
        if result.stderr: log_msg(f"   STDERR from fund_wallets.py:\n{result.stderr.strip()}", level=logging.WARNING, to_console=False)
    except subprocess.CalledProcessError as e:
        log_msg("   ❌ Error executing fund_wallets.py", level=logging.ERROR)
        if e.stdout: log_msg(f"   STDOUT: {e.stdout.strip()}", level=logging.ERROR, to_console=False)
        if e.stderr: log_msg(f"   STDERR: {e.stderr.strip()}", level=logging.ERROR, to_console=False)
        return False
    
    log_msg("   ✅ Wallet funding complete for this batch.")
    return True

def main():
    parser = argparse.ArgumentParser(description="Recursively register and fund wallets using binary expansion.")
    parser.add_argument("--fund-start", type=int, required=True, help="Initial funding start index")
    parser.add_argument("--fund-end", type=int, required=True, help="Initial funding end index")
    parser.add_argument("-s", "--dest-start", type=int, required=True, help="Destination start index")
    parser.add_argument("-e", "--dest-end", type=int, required=True, help="Destination end index")
    parser.add_argument("--night-amount", type=float, default=NIGHT_AMOUNT, help="Target NIGHT amount for each final wallet")
    parser.add_argument("--logfile", type=str, default=f"log_{int(time.time())}.txt", help="Path to store all stdout and stderr logs.")
    parser.add_argument("--node-url", type=str, default=NODE_URL, help="Node URL to fetch state from.")
    args = parser.parse_args()

    # Setup logging
    setup_logging(args.logfile)

    # Validate ranges
    if args.fund_start > args.fund_end:
        log_msg(f"❌ Error: --fund-start ({args.fund_start}) cannot be greater than --fund-end ({args.fund_end})", level=logging.ERROR)
        sys.exit(1)
    if args.dest_start > args.dest_end:
        log_msg(f"ℹ️  Info: --dest-start ({args.dest_start}) is greater than --dest-end ({args.dest_end}). Nothing to do.", level=logging.INFO)
        sys.exit(0)

    # Check for required scripts
    for script_name in ["register_dust.py", "fund_wallets.py"]:
        script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), script_name)
        if not os.path.exists(script_path):
            log_msg(f"❌ Error: Required script {script_path} not found.", level=logging.ERROR)
            sys.exit(1)

    # 1. Plan the batches using binary expansion
    batches = []
    current_fund_start = args.fund_start
    current_fund_end = args.fund_end
    next_dest_start = args.dest_start

    while next_dest_start <= args.dest_end:
        num_sources = current_fund_end - current_fund_start + 1
        batch_size = num_sources
        
        batch_dest_end = next_dest_start + batch_size - 1
        if batch_dest_end > args.dest_end:
            batch_dest_end = args.dest_end

        batches.append({
            "fund_start": current_fund_start,
            "fund_end": current_fund_end,
            "dest_start": next_dest_start,
            "dest_end": batch_dest_end
        })

        # The funding pool for the next batch expands to include the newly funded wallets.
        current_fund_end = batch_dest_end
        next_dest_start = batch_dest_end + 1

    log_msg(f"📋 Planned {len(batches)} batches.")

    # 2. Calculate required amounts for each batch in reverse order
    batch_amounts = [0.0] * len(batches)
    cumulative_future_cost = 0.0
    target_amount = args.night_amount

    for i in range(len(batches) - 1, -1, -1):
        # A wallet needs enough for its own target amount, plus the total amount it will
        # need to spend as a funding source in all subsequent batches.
        required_amount = target_amount + cumulative_future_cost
        batch_amounts[i] = required_amount
        cumulative_future_cost += required_amount

    # 3. Execute the planned batches
    log_msg(f"💰 Target Amount for final wallets: {target_amount} NIGHT")
    initial_req = target_amount + cumulative_future_cost
    log_msg(f"ℹ️  Initial funding seeds ({args.fund_start}-{args.fund_end}) need at least: {initial_req:.2f} NIGHT each.")
    log_msg("-" * 40)

    failed_batches = []
    for i, batch in enumerate(batches):
        amount = batch_amounts[i]
        log_msg(f"🚀 Batch {i+1}/{len(batches)}: Registering and Funding wallets {batch['dest_start']}-{batch['dest_end']}")
        
        log_msg(f"🔄 Fetching latest chain state from {args.node_url}...")
        try:
            fetch_cmd = [TOOLKIT_CMD, "fetch", "-s", args.node_url]
            log_msg(f"   Running: {' '.join(fetch_cmd)}", to_console=False)
            result = subprocess.run(fetch_cmd, check=True, capture_output=True, text=True)
            if result.stdout: log_msg(f"   STDOUT from fetch:\n{result.stdout.strip()}", to_console=False)
            if result.stderr: log_msg(f"   STDERR from fetch:\n{result.stderr.strip()}", level=logging.WARNING, to_console=False)
        except (subprocess.CalledProcessError, FileNotFoundError) as e:
            log_msg("   ❌ Error executing toolkit fetch", level=logging.ERROR)
            if hasattr(e, 'stdout') and e.stdout: log_msg(f"   STDOUT: {e.stdout.strip()}", level=logging.ERROR, to_console=False)
            if hasattr(e, 'stderr') and e.stderr: log_msg(f"   STDERR: {e.stderr.strip()}", level=logging.ERROR, to_console=False)
            log_msg(f"⚠️  Batch {i+1}/{len(batches)} failed during state fetch. Halting execution.", level=logging.WARNING)
            failed_batches.append(f"Batch {i+1} ({batch['dest_start']}-{batch['dest_end']}) - Fetch failed")
            break
        
        success = run_batch_actions(
            batch['fund_start'], 
            batch['fund_end'], 
            batch['dest_start'], 
            batch['dest_end'], 
            amount,
            args.node_url
        )
        if success:
            log_msg(f"✅ Batch {i+1}/{len(batches)} complete.\n")
        else:
            log_msg(f"⚠️  Batch {i+1}/{len(batches)} failed. Halting execution.", level=logging.WARNING)
            failed_batches.append(f"Batch {i+1} ({batch['dest_start']}-{batch['dest_end']})")
            # Stop if a batch fails, as subsequent batches depend on it.
            break
        
        if i < len(batches) - 1:
            print("\n⏳ Waiting for 12 seconds to allow for some dust accumulation")
            time.sleep(12)

    if failed_batches:
        log_msg("\n❌ Summary: The following batch failed:", level=logging.ERROR)
        for fb in failed_batches:
            log_msg(f"  - {fb}", level=logging.ERROR)
        log_msg("Subsequent batches were not attempted.", level=logging.ERROR)
        sys.exit(1)
    else:
        log_msg("\n🎉 All batches completed successfully.")

if __name__ == "__main__":
    main()