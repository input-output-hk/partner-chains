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
FUND_START = 1
FUND_END = 3
DEST_START = 4
DEST_END = 500

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

def run_command_with_streaming(cmd):
    """Runs a command, streaming stdout/stderr to console and log file."""
    # Ensure unbuffered output for Python scripts so logs appear immediately
    if cmd[0] == sys.executable and "-u" not in cmd:
        cmd.insert(1, "-u")

    process = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1
    )

    for line in process.stdout:
        line = line.rstrip()
        print(line)
        logging.info(line)

    return_code = process.wait()
    if return_code != 0:
        raise subprocess.CalledProcessError(return_code, cmd)

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

def run_batch_actions(fund_indices, dest_indices, amount, node_url, verbose=False, check_balances=False):
    """Runs dust registration and then funds the wallets for a given batch."""
    indices_str = format_indices_string(dest_indices)

    # Convert lists of ints to strings for the command line
    fund_indices_str = [str(i) for i in fund_indices]
    dest_indices_str = [str(i) for i in dest_indices]

    # --- Register Dust ---
    log_msg(f"-> Step 1: Registering dust for wallets {indices_str}")
    register_script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "register_dust.py")
    register_cmd = [
        sys.executable, register_script_path,
        "--fund-indices", *fund_indices_str,
        "--dest-indices", *dest_indices_str,
        "--node-url", node_url,
    ]
    if verbose:
        register_cmd.append("--verbose")
    if check_balances:
        register_cmd.append("--check-balances")

    log_msg(f"   Running: {' '.join(register_cmd)}")
    try:
        run_command_with_streaming(register_cmd)
    except subprocess.CalledProcessError:
        log_msg("   ❌ Error executing register_dust.py", level=logging.ERROR)
        return False

    log_msg("   ✅ Dust registration complete for this batch.")
    time.sleep(1) # Small delay between steps

    # --- Fund Wallets ---
    log_msg(f"-> Step 2: Funding wallets {indices_str} with {amount/1_000_000:.2f} NIGHT")
    fund_script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "fund_wallets.py")
    fund_cmd = [
        sys.executable, fund_script_path,
        "--fund-indices", *fund_indices_str,
        "--dest-indices", *dest_indices_str,
        "-a", str(amount / 1_000_000), # fund_wallets expects NIGHT, not smallest unit
        "--node-url", node_url,
    ]
    if verbose:
        fund_cmd.append("--verbose")
    if check_balances:
        fund_cmd.append("--check-balances")

    log_msg(f"   Running: {' '.join(fund_cmd)}")
    try:
        run_command_with_streaming(fund_cmd)
    except subprocess.CalledProcessError:
        log_msg("   ❌ Error executing fund_wallets.py", level=logging.ERROR)
        return False

    log_msg("   ✅ Wallet funding complete for this batch.")
    return True

def main():
    parser = argparse.ArgumentParser(description="Recursively register and fund wallets using binary expansion.")
    parser.add_argument("--fund-start", type=int, default=FUND_START, help="Initial funding start index")
    parser.add_argument("--fund-end", type=int, default=FUND_END, help="Initial funding end index")
    parser.add_argument("-s", "--dest-start", type=int, default=DEST_START, help="Destination start index")
    parser.add_argument("-e", "--dest-end", type=int, default=DEST_END, help="Destination end index")
    parser.add_argument("--fund-indices", nargs='+', help="List of specific funding seed indices (space or comma-separated)")
    parser.add_argument("-i", "--dest-indices", nargs='+', help="List of specific destination seed indices (space or comma-separated)")
    parser.add_argument("-a", "--night-amount", type=float, default=NIGHT_AMOUNT, help="Target NIGHT amount for each final wallet")
    parser.add_argument("--logfile", type=str, default=f"log_{int(time.time())}.txt", help="Path to store all stdout and stderr logs.")
    parser.add_argument("-v", "--verbose", action="store_true", help="Enable verbose output")
    parser.add_argument("--node-url", type=str, default=NODE_URL, help="Node URL to fetch state from.")
    parser.add_argument("--check-balances", action="store_true", help="Perform balance checks (default: False)")
    args = parser.parse_args()

    # Setup logging
    setup_logging(args.logfile)

    # Resolve funding indices
    funding_indices = []
    if args.fund_indices:
        for item in args.fund_indices:
            try:
                funding_indices.extend([int(i.strip()) for i in item.split(',') if i.strip()])
            except ValueError:
                log_msg(f"❌ Error: Invalid value in --fund-indices: '{item}'", level=logging.ERROR)
                sys.exit(1)
    elif args.fund_start is not None and args.fund_end is not None:
        if args.fund_start > args.fund_end:
            log_msg(f"❌ Error: --fund-start ({args.fund_start}) cannot be greater than --fund-end ({args.fund_end})", level=logging.ERROR)
            sys.exit(1)
        funding_indices = list(range(args.fund_start, args.fund_end + 1))
    else:
        log_msg("❌ Error: You must provide either --fund-indices or both --fund-start and --fund-end.", level=logging.ERROR)
        sys.exit(1)

    # Resolve destination indices
    all_dest_indices = []
    if args.dest_indices:
        for item in args.dest_indices:
            try:
                all_dest_indices.extend([int(i.strip()) for i in item.split(',') if i.strip()])
            except ValueError:
                log_msg(f"❌ Error: Invalid value in --dest-indices: '{item}'", level=logging.ERROR)
                sys.exit(1)
    elif args.dest_start is not None and args.dest_end is not None:
        if args.dest_start > args.dest_end:
            log_msg(f"ℹ️  Info: --dest-start ({args.dest_start}) is greater than --dest-end ({args.dest_end}). Nothing to do.", level=logging.INFO)
            sys.exit(0)
        all_dest_indices = list(range(args.dest_start, args.dest_end + 1))
    else:
        log_msg("❌ Error: You must provide either --dest-indices or both --dest-start and --dest-end.", level=logging.ERROR)
        sys.exit(1)

    # Check for required scripts
    for script_name in ["register_dust.py", "fund_wallets.py"]:
        script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), script_name)
        if not os.path.exists(script_path):
            log_msg(f"❌ Error: Required script {script_path} not found.", level=logging.ERROR)
            sys.exit(1)

    # 1. Plan the batches using binary expansion
    batches = []
    # We work with a copy of dest indices to slice from
    remaining_dest_indices = list(all_dest_indices)
    current_funding_indices = list(funding_indices)

    while remaining_dest_indices:
        num_sources = len(current_funding_indices)
        batch_size = num_sources

        batch_dest_indices = remaining_dest_indices[:batch_size]
        remaining_dest_indices = remaining_dest_indices[batch_size:]

        batches.append({
            "fund_indices": list(current_funding_indices), # Store a copy for this batch
            "dest_indices": batch_dest_indices,
        })

        # The funding pool for the next batch expands to include the newly funded wallets.
        current_funding_indices.extend(batch_dest_indices)

    log_msg(f"📋 Planned {len(batches)} batches.")

    # 2. Calculate required amounts for each batch in reverse order
    batch_amounts = [0.0] * len(batches)
    cumulative_future_cost = 0.0
    target_amount = args.night_amount * 1_000_000 # Convert to smallest unit

    for i in range(len(batches) - 1, -1, -1):
        # A wallet needs enough for its own target amount, plus the total amount it will
        # need to spend as a funding source in all subsequent batches.
        required_amount = target_amount + cumulative_future_cost
        batch_amounts[i] = required_amount
        cumulative_future_cost += required_amount

    # 3. Execute the planned batches
    log_msg(f"💰 Target Amount for final wallets: {args.night_amount} NIGHT")
    initial_req = target_amount + cumulative_future_cost

    if args.fund_start is not None and args.fund_end is not None:
        log_msg(f"ℹ️  Initial funding seeds ({args.fund_start}-{args.fund_end}) need at least: {initial_req/1_000_000:.2f} NIGHT each.")
    else:
        log_msg(f"ℹ️  Initial funding seeds (count: {len(funding_indices)}) need at least: {initial_req/1_000_000:.2f} NIGHT each.")

    log_msg("-" * 40)

    # Start timing
    start_time = time.time()

    failed_batches = []
    for i, batch in enumerate(batches):
        amount = batch_amounts[i]
        indices_str = format_indices_string(batch['dest_indices'])
        log_msg(f"🚀 Batch {i+1}/{len(batches)}: Registering and Funding wallets {indices_str}")

        log_msg(f"��🔄 Fetching latest chain state from {args.node_url}...")
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
            failed_batches.append(f"Batch {i+1} ({indices_str}) - Fetch failed")
            break

        success = run_batch_actions(
            batch['fund_indices'], 
            batch['dest_indices'], 
            amount,
            args.node_url,
            verbose=args.verbose,
            check_balances=args.check_balances
        )
        if success:
            log_msg(f"✅ Batch {i+1}/{len(batches)} complete.\n")
        else:
            log_msg(f"⚠️  Batch {i+1}/{len(batches)} failed. Halting execution.", level=logging.WARNING)
            failed_batches.append(f"Batch {i+1} ({indices_str})")
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
        # Calculate total execution time
        total_time = time.time() - start_time
        minutes = int(total_time // 60)
        seconds = int(total_time % 60)

        if minutes > 0:
            time_str = f"{minutes} minute{'s' if minutes != 1 else ''} {seconds} second{'s' if seconds != 1 else ''}"
        else:
            time_str = f"{seconds} second{'s' if seconds != 1 else ''}"

        log_msg(f"\n🎉 All batches completed successfully in {time_str}.")

if __name__ == "__main__":
    main()