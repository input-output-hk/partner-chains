#!/usr/bin/env python3
import argparse
import subprocess
import sys
import os
import time

# Configuration
# Assumes the applied script is in the same directory as this script
APPLIED_SCRIPT = "fund_wallets"

def run_applied_script(script_name, fund_start, fund_end, target_start, target_end, amount):
    script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), f"{script_name}.py")
    cmd = [
        sys.executable, script_path,
        "--fund-start", str(fund_start),
        "--fund-end", str(fund_end),
        "--dest-start", str(target_start),
        "--dest-end", str(target_end)
    ]

    if script_name == "fund_wallets":
        cmd.extend(["--night-amount", str(amount)])

    print(f"Running: {' '.join(cmd)}")
    try:
        subprocess.run(cmd, check=True)
        return True
    except subprocess.CalledProcessError:
        print(f"❌ Error executing {script_name}.py")
        return False

def main():
    parser = argparse.ArgumentParser(description="Recursively fund wallets using binary expansion.")
    parser.add_argument("--fund-start", type=int, required=True, help="Initial funding start index")
    parser.add_argument("--fund-end", type=int, required=True, help="Initial funding end index")
    parser.add_argument("--dest-start", type=int, required=True, help="Destination start index")
    parser.add_argument("--dest-end", type=int, required=True, help="Destination end index")
    parser.add_argument("--night-amount", type=float, required=True, help="Target NIGHT amount per wallet")
    parser.add_argument("--script", type=str, default=APPLIED_SCRIPT, help="Script to run (fund_wallets or register_dust)")
    args = parser.parse_args()

    script_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), f"{args.script}.py")
    if not os.path.exists(script_path):
        print(f"❌ Error: {script_path} not found.")
        sys.exit(1)

    # 1. Plan the batches
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
        
        current_fund_end = batch_dest_end
        next_dest_start = batch_dest_end + 1

    print(f"📋 Planned {len(batches)} batches.")

    # 2. Calculate required amounts (Reverse order)
    # A wallet funded in batch `i` needs to cover costs for batches `i+1` to `N` where it acts as source.
    # It acts as source in ALL subsequent batches.
    # Cost per batch as source = Amount_for_that_batch + Fee.
    
    batch_amounts = [0.0] * len(batches)
    cumulative_future_cost = 0.0
    
    if args.script == "register_dust":
        target_amount = 0.0
    else:
        target_amount = args.night_amount
    
    for i in range(len(batches) - 1, -1, -1):
        required_amount = target_amount + cumulative_future_cost
        batch_amounts[i] = required_amount
        cumulative_future_cost += required_amount

    # 3. Execute
    print(f"💰 Target Amount: {target_amount} NIGHT")
    
    initial_req = target_amount + cumulative_future_cost
    print(f"ℹ️  Initial funding seeds ({args.fund_start}-{args.fund_end}) need at least: {initial_req:.2f} NIGHT each.")
    print("-" * 40)
    
    failed_batches = []
    for i, batch in enumerate(batches):
        amount = batch_amounts[i]
        if args.script == "fund_wallets":
            print(f"🚀 Batch {i+1}/{len(batches)}: Funding {batch['dest_start']}-{batch['dest_end']} with {amount:.2f} NIGHT")
        else:
            print(f"🚀 Batch {i+1}/{len(batches)}: Registering Dust on {batch['dest_start']}-{batch['dest_end']}")
        success = run_applied_script(
            args.script,
            batch['fund_start'], 
            batch['fund_end'], 
            batch['dest_start'], 
            batch['dest_end'], 
            amount
        )
        if success:
            print("✅ Batch complete.\n")
        else:
            print("⚠️  Batch failed. Continuing...\n")
            failed_batches.append(f"Batch {i+1} ({batch['dest_start']}-{batch['dest_end']})")
        time.sleep(2)

    if failed_batches:
        print("\n❌ Summary: The following batches failed:")
        for fb in failed_batches:
            print(f"  - {fb}")
        sys.exit(1)
    else:
        print("\n🎉 All batches completed successfully.")

if __name__ == "__main__":
    main()