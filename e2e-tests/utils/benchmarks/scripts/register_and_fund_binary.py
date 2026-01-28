#!/usr/bin/env python3
import argparse
import subprocess
import sys
import os
import time

def run_batch_actions(fund_start, fund_end, dest_start, dest_end, amount):
    """Runs dust registration and then funds the wallets for a given batch."""
    # --- Register Dust ---
    print(f"-> Step 1: Registering dust for wallets {dest_start}-{dest_end}")
    register_script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "register_dust.py")
    register_cmd = [
        sys.executable, register_script_path,
        "--fund-start", str(fund_start),
        "--fund-end", str(fund_end),
        "--dest-start", str(dest_start),
        "--dest-end", str(dest_end)
    ]
    print(f"   Running: {' '.join(register_cmd)}")
    try:
        # Using capture_output to keep the main script's log clean unless there's an error.
        subprocess.run(register_cmd, check=True, capture_output=True, text=True)
    except subprocess.CalledProcessError as e:
        print(f"   ❌ Error executing register_dust.py")
        print(f"   STDOUT: {e.stdout}")
        print(f"   STDERR: {e.stderr}")
        return False

    print("   ✅ Dust registration complete for this batch.")
    time.sleep(6) # 1 block delay between steps

    # --- Fund Wallets ---
    print(f"-> Step 2: Funding wallets {dest_start}-{dest_end} with {amount:.2f} NIGHT")
    fund_script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "fund_wallets.py")
    fund_cmd = [
        sys.executable, fund_script_path,
        "--fund-start", str(fund_start),
        "--fund-end", str(fund_end),
        "--dest-start", str(dest_start),
        "--dest-end", str(dest_end),
        "--night-amount", str(amount)
    ]
    print(f"   Running: {' '.join(fund_cmd)}")
    try:
        subprocess.run(fund_cmd, check=True, capture_output=True, text=True)
    except subprocess.CalledProcessError as e:
        print(f"   ❌ Error executing fund_wallets.py")
        print(f"   STDOUT: {e.stdout}")
        print(f"   STDERR: {e.stderr}")
        return False
    
    print("   ✅ Wallet funding complete for this batch.")
    return True

def main():
    parser = argparse.ArgumentParser(description="Recursively register and fund wallets using binary expansion.")
    parser.add_argument("--fund-start", type=int, required=True, help="Initial funding start index")
    parser.add_argument("--fund-end", type=int, required=True, help="Initial funding end index")
    parser.add_argument("--dest-start", type=int, required=True, help="Destination start index")
    parser.add_argument("--dest-end", type=int, required=True, help="Destination end index")
    parser.add_argument("--night-amount", type=float, required=True, help="Target NIGHT amount for each final wallet")
    args = parser.parse_args()

    # Check for required scripts
    for script_name in ["register_dust.py", "fund_wallets.py"]:
        script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), script_name)
        if not os.path.exists(script_path):
            print(f"❌ Error: Required script {script_path} not found.")
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

    print(f"📋 Planned {len(batches)} batches.")

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
    print(f"💰 Target Amount for final wallets: {target_amount} NIGHT")
    initial_req = target_amount + cumulative_future_cost
    print(f"ℹ️  Initial funding seeds ({args.fund_start}-{args.fund_end}) need at least: {initial_req:.2f} NIGHT each.")
    print("-" * 40)

    failed_batches = []
    for i, batch in enumerate(batches):
        amount = batch_amounts[i]
        print(f"🚀 Batch {i+1}/{len(batches)}: Registering and Funding wallets {batch['dest_start']}-{batch['dest_end']}")
        
        success = run_batch_actions(
            batch['fund_start'], 
            batch['fund_end'], 
            batch['dest_start'], 
            batch['dest_end'], 
            amount
        )
        if success:
            print(f"✅ Batch {i+1}/{len(batches)} complete.\n")
        else:
            print(f"⚠️  Batch {i+1}/{len(batches)} failed. Halting execution.")
            failed_batches.append(f"Batch {i+1} ({batch['dest_start']}-{batch['dest_end']})")
            # Stop if a batch fails, as subsequent batches depend on it.
            break
        
        if i < len(batches) - 1:
            time.sleep(6)

    if failed_batches:
        print("\n❌ Summary: The following batch failed:")
        for fb in failed_batches:
            print(f"  - {fb}")
        print("Subsequent batches were not attempted.")
        sys.exit(1)
    else:
        print("\n🎉 All batches completed successfully.")

if __name__ == "__main__":
    main()
