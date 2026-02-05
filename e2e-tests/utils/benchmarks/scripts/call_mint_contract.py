#!/usr/bin/env python3
import subprocess
import argparse
import os
import sys
import json
import random
import time

# Default Configuration from the shell script
DEFAULT_SEED = "0000000000000000000000000000000000000000000000000000000000000001"
DEFAULT_NONCE = "2339000000003000000000000000000000000000000000000000000000000000"
DEFAULT_DOMAIN_SEP = "beeb000000040000000000000000000000000000000000000000000000000000"
DEFAULT_AMOUNT = str(int(1000000 * random.uniform(0.9, 1.1)))
DEFAULT_NETWORK = "undeployed"
DEFAULT_NODE_URL = "ws://henry.node.sc.iog.io:9944"
DEFAULT_WORK_DIR = "./e2e-tests/utils/benchmarks/mint_contract_state_files"
DEFAULT_TOOLKIT_CMD = "midnight-node-toolkit"
DEFAULT_TOOLKIT_JS_PATH = "/home/christos/shielded/midnight-node/util/toolkit-js"

def run_command(cmd, cwd=None, verbose=True, env=None):
    """Runs a command and returns stdout if successful, exits otherwise."""
    if verbose:
        print(f"Running: {' '.join(cmd)}")
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True, cwd=cwd, env=env)
        if verbose:
            if result.stderr:
                print(f"STDERR: {result.stderr.strip()}")
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        print(f"\n❌ Error executing command: {' '.join(cmd)}")
        print(f"STDOUT: {e.stdout}")
        print(f"STDERR: {e.stderr}")
        sys.exit(1)
    except FileNotFoundError:
        print(f"\n❌ Error: Executable '{cmd[0]}' not found.")
        sys.exit(1)

def main():
    start_time = time.time()
    print(f"--- Script started at {time.ctime(start_time)} ---")

    parser = argparse.ArgumentParser(description="Call mint contract script.")
    parser.add_argument("--seed", default=DEFAULT_SEED, help="Wallet seed")
    parser.add_argument("--nonce", default=DEFAULT_NONCE, help="Contract nonce")
    parser.add_argument("--domain-sep", default=DEFAULT_DOMAIN_SEP, help="Domain separator")
    parser.add_argument("--amount", default=DEFAULT_AMOUNT, help="Amount to mint")
    parser.add_argument("--network", default=DEFAULT_NETWORK, help="Network name")
    parser.add_argument("--node-url", default=DEFAULT_NODE_URL, help="Node URL")
    parser.add_argument("--work-dir", default=DEFAULT_WORK_DIR, help="Working directory containing artifacts")
    parser.add_argument("--toolkit-cmd", default=DEFAULT_TOOLKIT_CMD, help="Path to midnight-node-toolkit")
    parser.add_argument("--toolkit-js-path", default=DEFAULT_TOOLKIT_JS_PATH, help="Path to toolkit-js directory")
    parser.add_argument("--verbose", action="store_true", help="Verbose output")

    args = parser.parse_args()

    # Ensure work dir exists
    if not os.path.exists(args.work_dir):
        os.makedirs(args.work_dir)

    # 1. Derive wallet addresses
    print("--- Deriving wallet addresses ---")
    cmd_coin_public = [
        args.toolkit_cmd, "show-address",
        "--network", args.network,
        "--seed", args.seed,
        "--coin-public"
    ]
    coin_public = run_command(cmd_coin_public, verbose=args.verbose)
    print(f"Coin Public: {coin_public}")

    cmd_shielded = [
        args.toolkit_cmd, "show-address",
        "--network", args.network,
        "--seed", args.seed,
        "--shielded"
    ]
    shielded_dest = run_command(cmd_shielded, verbose=args.verbose)
    print(f"Shielded Dest: {shielded_dest}")

    # 2. Extract contract address
    print("\n--- Extracting contract address ---")
    deploy_tx_path = os.path.join(args.work_dir, "deploy_tx.mn")
    if not os.path.exists(deploy_tx_path):
         print(f"❌ deploy_tx.mn not found at {deploy_tx_path}")
         sys.exit(1)

    cmd_contract_addr = [
        args.toolkit_cmd, "contract-address",
        "--src-file", deploy_tx_path
    ]
    contract_address = run_command(cmd_contract_addr, verbose=args.verbose)
    print(f"Contract Address: {contract_address}")


    # 3. Fetch contract state path
    contract_state_path = os.path.join(args.work_dir, "contract_state.mn")

    if not os.path.exists(contract_state_path):
        print(f"--- Fetching contract state ---")
        cmd_contract_state = [
            args.toolkit_cmd, "contract-state",
            "--contract-address", contract_address,
            "-d", contract_state_path,
            "-s", args.node_url
        ]
        run_command(cmd_contract_state, verbose=args.verbose)

    # Derive paths from toolkit-js-path
    mint_config_file = os.path.join(args.toolkit_js_path, "mint", "mint.config.ts")
    mint_out_dir = os.path.join(args.toolkit_js_path, "mint", "out")

    # 4. Generate mint circuit proof
    print(f"\n--- Generating circuit proof for minting {args.amount} tokens ---")
    
    # We need absolute paths for inputs since we are changing CWD for the subprocess
    abs_contract_state = os.path.abspath(contract_state_path)
    abs_private_state = os.path.abspath(os.path.join(args.work_dir, "private_state.json"))
    abs_mint_bin = os.path.abspath(os.path.join(args.work_dir, "mint.bin"))
    abs_private_state2 = os.path.abspath(os.path.join(args.work_dir, "private_state2.json"))
    if not os.path.exists(abs_private_state2):
        with open(abs_private_state2, "w") as f:
            f.write("{}")
    abs_mint_zswap = os.path.abspath(os.path.join(args.work_dir, "mint_zswap.json"))

    cmd_circuit = [
        args.toolkit_cmd, "generate-intent", "circuit",
        "--config", mint_config_file,
        "--toolkit-js-path", args.toolkit_js_path,
        "--input-onchain-state", abs_contract_state,
        "--input-private-state", abs_private_state,
        "--contract-address", contract_address,
        "--output-intent", abs_mint_bin,
        "--output-private-state", abs_private_state2,
        "--output-zswap-state", abs_mint_zswap,
        "--coin-public", coin_public,
        "mint", args.nonce, args.domain_sep, args.amount
    ]
    
    run_command(cmd_circuit, verbose=args.verbose)

    # 5. Submit mint transaction
    print("\n--- Submitting mint transaction ---")
    # Ensure compiled_contract_dir is absolute or relative to CWD (which is script dir now)
    # Since we are running toolkit from CWD, we should probably make this absolute to be safe
    if not os.path.exists(mint_out_dir):
        os.makedirs(mint_out_dir)
    abs_compiled_contract_dir = os.path.abspath(mint_out_dir)

    cmd_send_intent = [
        args.toolkit_cmd, "send-intent",
        "--intent-file", abs_mint_bin,
        "--zswap-state-file", abs_mint_zswap,
        "--compiled-contract-dir", abs_compiled_contract_dir,
        "--shielded-destination", shielded_dest,
        "-s", args.node_url,
        "-d", args.node_url
    ]
    run_command(cmd_send_intent, verbose=args.verbose)

    # 6. Display results
    print("\n--- Results ---")
    cmd_token_type = [
        args.toolkit_cmd, "show-token-type",
        "--contract-address", contract_address,
        "--domain-sep", args.domain_sep,
        "--shielded"
    ]
    token_type = run_command(cmd_token_type, verbose=args.verbose)
    # Ensure we get just the hex string if there are logs in stdout
    if "\n" in token_type:
        token_type = token_type.splitlines()[-1]
    
    print(f"Contract: {contract_address}")
    print(f"Token type: {token_type}")
    print("Checking wallet for minted tokens:")
    
    cmd_show_wallet = [
        args.toolkit_cmd, "show-wallet",
        "--seed", args.seed,
        "-s", args.node_url
    ]
    wallet_output = run_command(cmd_show_wallet, verbose=args.verbose)

    try:
        # Find the start of the JSON output (skip logs)
        json_start = wallet_output.find('{')
        if json_start != -1:
            wallet_output = wallet_output[json_start:]

        wallet_data = json.loads(wallet_output)
        coins = wallet_data.get("coins", {})
        matching_coins = {}
        target_amount = int(args.amount)

        for coin_id, coin_data in coins.items():
            if (coin_data.get("nonce") == args.nonce and
                coin_data.get("token_type") == token_type and
                coin_data.get("value") == target_amount):
                matching_coins[coin_id] = coin_data

        print(json.dumps(matching_coins, indent=2))
    except json.JSONDecodeError:
        print(f"❌ Failed to parse wallet JSON. Output:\n{wallet_output}")

    end_time = time.time()
    print(f"\n--- Script ended at {time.ctime(end_time)} ---")
    duration = end_time - start_time
    minutes = int(duration // 60)
    seconds = int(duration % 60)
    print(f"⏱️ Total time: {minutes} min {seconds} seconds")

if __name__ == "__main__":
    main()