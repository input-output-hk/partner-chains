#! /bin/bash

echo "[LOG] Script start. Setting permissions and initial variables."
chmod 600 /keys/*
chmod +x /busybox
chmod 777 /shared

echo "[LOG] Calculating target time for synchronised chain start..."

# Local env Partner Chains epochs are 30 seconds long. PC and MC epochs have to align. The following line makes MC epoch 0 start at some PC epoch start.
target_time=$(( ($(date +%s) / 30 + 1) * 30 ))
echo "$target_time" > /shared/cardano.start
byron_startTime=$target_time
shelley_systemStart=$(date --utc +"%Y-%m-%dT%H:%M:%SZ" --date="@$target_time")

echo "[LOG] Target time calculated: $target_time. Byron StartTime: $byron_startTime, Shelley systemStart: $shelley_systemStart"

/busybox sed "s/\"startTime\": [0-9]*/\"startTime\": $byron_startTime/" /shared/byron/genesis.json.base > /shared/byron/genesis.json
echo "Updated startTime value in Byron genesis.json to: $byron_startTime"

/busybox sed "s/\"systemStart\": \"[^\"]*\"/\"systemStart\": \"$shelley_systemStart\"/" /shared/shelley/genesis.json.base > /shared/shelley/genesis.json
echo "Updated systemStart value in Shelley genesis.json to: $shelley_systemStart"

echo "[LOG] Updated Byron and Shelley genesis files with new start times."

echo "Parsing epochLength and slotLength from Shelley genesis.json..."
/busybox awk -F':|,' '/"epochLength"/ {print $2}' /shared/shelley/genesis.json.base > /shared/mc-epoch-length
echo "Created /shared/mc-epoch-length with value: $(cat /shared/mc-epoch-length)"

/busybox awk -F':|,' '/"slotLength"/ {print $2}' /shared/shelley/genesis.json.base > /shared/mc-slot-length
echo "Created /shared/mc-slot-length with value: $(cat /shared/mc-slot-length)"

echo "[LOG] Extracted mc-epoch-length and mc-slot-length."

cp /shared/conway/genesis.conway.json.base /shared/conway/genesis.conway.json
cp /shared/shelley/genesis.alonzo.json.base /shared/shelley/genesis.alonzo.json
echo "Created /shared/conway/genesis.conway.json and /shared/shelley/genesis.alonzo.json"

echo "[LOG] Copied Conway and Alonzo genesis files."

byron_hash=$(/bin/cardano-cli byron genesis print-genesis-hash --genesis-json /shared/byron/genesis.json)
shelley_hash=$(/bin/cardano-cli latest genesis hash --genesis /shared/shelley/genesis.json)
alonzo_hash=$(/bin/cardano-cli latest genesis hash --genesis /shared/shelley/genesis.alonzo.json)
conway_hash=$(/bin/cardano-cli latest genesis hash --genesis /shared/conway/genesis.conway.json)

echo "[LOG] Calculated Byron, Shelley, Alonzo, Conway genesis hashes."

/busybox sed "s/\"ByronGenesisHash\": \"[^\"]*\"/\"ByronGenesisHash\": \"$byron_hash\"/" /shared/node-1-config.json.base > /shared/node-1-config.json.base.byron
/busybox sed "s/\"ByronGenesisHash\": \"[^\"]*\"/\"ByronGenesisHash\": \"$byron_hash\"/" /shared/db-sync-config.json.base > /shared/db-sync-config.json.base.byron
/busybox sed "s/\"ShelleyGenesisHash\": \"[^\"]*\"/\"ShelleyGenesisHash\": \"$shelley_hash\"/" /shared/node-1-config.json.base.byron > /shared/node-1-config.base.shelley
/busybox sed "s/\"ShelleyGenesisHash\": \"[^\"]*\"/\"ShelleyGenesisHash\": \"$shelley_hash\"/" /shared/db-sync-config.json.base.byron > /shared/db-sync-config.base.shelley
/busybox sed "s/\"AlonzoGenesisHash\": \"[^\"]*\"/\"AlonzoGenesisHash\": \"$alonzo_hash\"/" /shared/node-1-config.base.shelley > /shared/node-1-config.json.base.conway
/busybox sed "s/\"AlonzoGenesisHash\": \"[^\"]*\"/\"AlonzoGenesisHash\": \"$alonzo_hash\"/" /shared/db-sync-config.base.shelley > /shared/db-sync-config.json.base.conway
/busybox sed "s/\"ConwayGenesisHash\": \"[^\"]*\"/\"ConwayGenesisHash\": \"$conway_hash\"/" /shared/node-1-config.json.base.conway > /shared/node-1-config.json
/busybox sed "s/\"ConwayGenesisHash\": \"[^\"]*\"/\"ConwayGenesisHash\": \"$conway_hash\"/" /shared/db-sync-config.json.base.conway > /shared/db-sync-config.json

echo "Updated ByronGenesisHash value in config files to: $byron_hash"
echo "Updated ShelleyGenesisHash value in config files to: $shelley_hash"
echo "Updated ConwayGenesisHash value in config files to: $conway_hash"

echo "[LOG] Updated node and db-sync config files with genesis hashes."

byron_startTimeMillis=$(($byron_startTime * 1000))
echo $byron_startTimeMillis > /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS
echo $byron_startTimeMillis > /runtime-values/MC__FIRST_EPOCH_TIMESTAMP_MILLIS
echo "[LOG] Created /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS: $(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)"

echo "[LOG] Current time before starting node: $(date +\"%H:%M:%S.%3N\"). Starting Cardano node process..."

cardano-node run \
  --topology /shared/node-1-topology.json \
  --database-path /data/db \
  --socket-path /data/node.socket \
  --host-addr 0.0.0.0 \
  --port 32000 \
  --config /shared/node-1-config.json \
  --shelley-kes-key /keys/kes.skey \
  --shelley-vrf-key /keys/vrf.skey \
  --shelley-operational-certificate /keys/node.cert &

echo "[LOG] Waiting for node.socket at /data/node.socket..."

while true; do
    if [ -e "/data/node.socket" ]; then
        break
    else
        sleep 1
    fi
done

echo "[LOG] node.socket found."

echo "[LOG] Preparing native token and initial funding addresses."
# Policy requires that mints are signed by the funded_address.skey (key hash is e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b)
reward_token_policy_id=$(cardano-cli latest transaction policyid --script-file ./shared/reward_token_policy.script)
# hex of "Reward token"
reward_token_asset_name="52657761726420746f6b656e"
echo "[LOG] Native token policy ID: $reward_token_policy_id, Asset Name: $reward_token_asset_name"
echo "Generating new address and funding it with 2x1000 Ada and 10 Ada + 1000000 reward token ($reward_token_policy_id.$reward_token_asset_name)"

new_address=$(cardano-cli latest address build \
  --payment-verification-key-file /keys/funded_address.vkey \
  --testnet-magic 42)

echo "[LOG] New main address created: $new_address"

# Array to store payment addresses for registered nodes
registered_node_payment_addresses=()

echo "[LOG] Generating Payment Keypairs and Addresses for 300 Registered Nodes..."
for i in {1..300}; do
    NODE_SPECIFIC_KEYS_DIR="/shared/node-keys/registered-${i}/keys"
    mkdir -p "$NODE_SPECIFIC_KEYS_DIR" # Ensure cold key dir also exists if not created yet

    echo "[LOG] Generating payment keys for registered-$i in $NODE_SPECIFIC_KEYS_DIR..."
    cardano-cli address key-gen \
        --verification-key-file "${NODE_SPECIFIC_KEYS_DIR}/payment.vkey" \
        --signing-key-file "${NODE_SPECIFIC_KEYS_DIR}/payment.skey"
    
    if [ $? -ne 0 ]; then
        echo "Error generating payment keys for registered-$i!"
        exit 1
    fi

    node_payment_address=$(cardano-cli address build \
        --payment-verification-key-file "${NODE_SPECIFIC_KEYS_DIR}/payment.vkey" \
        --testnet-magic 42) # Assuming testnet-magic 42 from context

    if [ -z "$node_payment_address" ]; then
        echo "Error building payment address for registered-$i!"
        exit 1
    fi
    registered_node_payment_addresses+=("$node_payment_address")
    echo "Generated payment address for registered-$i: $node_payment_address"
done
echo "[LOG] Completed generation of payment keypairs and addresses for 300 registered nodes."

# An address that will keep an UTXO with script of a test V-function, related to the SPO rewards. See v-function.script file.
vfunction_address="addr_test1vzuasm5nqzh7n909f7wang7apjprpg29l2f9sk6shlt84rqep6nyc"

# Define the UTXO details and amounts
tx_in1="781cb948a37c7c38b43872af9b1e22135a94826eafd3740260a6db0a303885d8#0"
tx_in_amount=29993040000000000

# Define output amounts
# partner-chains-setup
tx_out1=1000000000
# partner-chains-setup (backup)
tx_out2=1000000000
# partner-chains-setup (additional)
tx_out3=1000000000
# partner-chains-setup (extra)
tx_out4=1000000000

# Fund 10 permissioned nodes
for i in {1..10}; do
    var_name="tx_out${i}_permissioned"
    declare "$var_name=1000000000"
done

# Fund 300 registered nodes (These are defined but not used in the *initial* main transaction anymore)
# They are funded in batches later.
for i in {1..300}; do
    var_name="tx_out${i}_registered"
    declare "$var_name=1000000000"
done

tx_out5_lovelace=10000000
tx_out5_reward_token="1000000 $reward_token_policy_id.$reward_token_asset_name"
tx_out6=10000000

# Calculate total output
echo "[LOG] Calculating initial total output (tx_out1-4)."
total_output=$((tx_out1 + tx_out2 + tx_out3 + tx_out4))
echo "[LOG] Initial total_output = $total_output"

echo "[LOG] Adding permissioned node amounts to total_output."
for i in {1..10}; do
    var_name="tx_out${i}_permissioned"
    echo "[DEBUG] var_name for permissioned loop iteration $i is: $var_name"
    echo "[DEBUG] Value of variable named '$var_name' is: $(eval echo "\$$var_name")"
    amount_permissioned="${!var_name}"
    echo "[DEBUG] amount_permissioned for iteration $i after dereference is: '$amount_permissioned'"
    total_output=$((total_output + amount_permissioned))
done
echo "[LOG] total_output after permissioned nodes = $total_output"

echo "[LOG] Adding tx_out5_lovelace and tx_out6 to total_output."
total_output=$((total_output + tx_out5_lovelace + tx_out6))
echo "[LOG] Final total_output before fee = $total_output"

echo "[LOG] Calculating total output for the main funding transaction." # This log might be slightly misplaced now but ok

# --- Main Transaction Dynamic Fee Calculation and Build ---
echo "[LOG] Assembling parameters for the main funding transaction."
main_tx_out_params_array=()
main_tx_out_params_array+=(--tx-out "$new_address+$tx_out1")
main_tx_out_params_array+=(--tx-out "$new_address+$tx_out2")
main_tx_out_params_array+=(--tx-out "$new_address+$tx_out3")
main_tx_out_params_array+=(--tx-out "$new_address+$tx_out4")

# Permissioned nodes outputs (still to $new_address)
for i in {1..10}; do
    var_name="tx_out${i}_permissioned"
    amount_permissioned="${!var_name}"
    main_tx_out_params_array+=(--tx-out "$new_address+$amount_permissioned")
done

# Output with native token for new_address
main_tx_out_params_array+=(--tx-out "$new_address+$tx_out5_lovelace+$tx_out5_reward_token")

# Output for vfunction_address (this one sends to a different address)
main_tx_out_params_array+=(--tx-out "$vfunction_address+$tx_out6")

# Calculate number of outputs for fee calculation (excluding change for now)
num_main_tx_outputs_before_change=$((${#main_tx_out_params_array[@]} / 2)) # Each --tx-out "addr+val" is 2 array elements for params

echo "[LOG] Querying protocol parameters for main transaction fee..."
protocol_params_file="/data/protocol.json" # Define it here or ensure it's defined if moved earlier
if [ ! -f "$protocol_params_file" ]; then # If not already queried by batch logic, or if that logic is moved
    if ! cardano-cli latest query protocol-parameters --testnet-magic 42 --out-file "$protocol_params_file"; then
        echo "[DEBUG] CRITICAL ERROR: Failed to query protocol parameters for main transaction. Using fallback fee."
    fi
fi

echo "[LOG] Building DUMMY main transaction for fee calculation..."
dummy_main_tx_file="/data/tx_main_dummy.raw"
dummy_change_placeholder=1000000 # Placeholder for change in dummy tx

temp_main_tx_out_params_array=("${main_tx_out_params_array[@]}")
temp_main_tx_out_params_array+=(--tx-out "$new_address+$dummy_change_placeholder")

if ! cardano-cli latest transaction build-raw \
  --tx-in "$tx_in1" \
  "${temp_main_tx_out_params_array[@]}" \
  --tx-out-reference-script-file /shared/v-function.script \
  --minting-script-file /shared/reward_token_policy.script \
  --mint "$tx_out5_reward_token" \
  --fee 0 \
  --out-file "$dummy_main_tx_file"; then
    echo "[DEBUG] Main Tx: ERROR building DUMMY transaction for fee calculation. Using fallback fee."
    main_tx_fee=300000 # Fallback fee
else
    main_tx_num_inputs=1
    main_tx_num_outputs=$((num_main_tx_outputs_before_change + 1)) # N outputs + 1 change output
    main_tx_num_witnesses=2 # genesis-utxo.skey and funded_address.skey (for minting)

    echo "[LOG] Main Tx: Calculating min fee. Inputs: $main_tx_num_inputs, Outputs: $main_tx_num_outputs, Witnesses: $main_tx_num_witnesses, Protocol File: $protocol_params_file"
    calculated_main_fee=$(cardano-cli latest transaction calculate-min-fee \
        --tx-body-file "$dummy_main_tx_file" \
        --testnet-magic 42 \
        --protocol-params-file "$protocol_params_file" \
        --tx-in-count "$main_tx_num_inputs" \
        --tx-out-count "$main_tx_num_outputs" \
        --witness-count "$main_tx_num_witnesses" | /busybox awk '{print $1}')

    if ! [[ "$calculated_main_fee" =~ ^[0-9]+$ ]] || [ -z "$calculated_main_fee" ]; then
        echo "[DEBUG] Main Tx: ERROR calculating dynamic fee (Raw output: '$calculated_main_fee'). Using fallback static fee 300000."
        main_tx_fee=300000
    else
        main_tx_fee=$((calculated_main_fee + 1000)) # Add a 1000 lovelace buffer
        echo "[LOG] Main Tx: Calculated Min Fee: $calculated_main_fee, Using Fee with Buffer: $main_tx_fee"
    fi
    rm -f "$dummy_main_tx_file"
fi

echo "[LOG] Main transaction: total_output_value=$total_output, fee=$main_tx_fee"

# Calculate remaining balance (change) for the main transaction
main_tx_change=$((tx_in_amount - total_output - main_tx_fee))
echo "[LOG] Main Tx: Change calculated: $main_tx_change (tx_in_amount=$tx_in_amount - total_output=$total_output - fee=$main_tx_fee)"

if [ "$main_tx_change" -lt 1000000 ]; then # Minimum change 1 ADA
    echo "[DEBUG] CRITICAL ERROR: Main transaction change is less than 1 ADA ($main_tx_change). Aborting."
    exit 1
fi

# Add the actual change output to the parameters array
main_tx_out_params_array+=(--tx-out "$new_address+$main_tx_change")

echo "[LOG] Building the FINAL main funding transaction raw file..."
if ! cardano-cli latest transaction build-raw \
  --tx-in "$tx_in1" \
  "${main_tx_out_params_array[@]}" \
  --tx-out-reference-script-file /shared/v-function.script \
  --minting-script-file /shared/reward_token_policy.script \
  --mint "$tx_out5_reward_token" \
  --fee "$main_tx_fee" \
  --out-file /data/tx.raw; then
    echo "[DEBUG] CRITICAL ERROR: Failed to build FINAL main transaction. Aborting."
    exit 1
fi
echo "[LOG] Main funding transaction raw file created at /data/tx.raw."

echo "[LOG] Signing the main funding transaction..."
if ! cardano-cli latest transaction sign \
  --tx-body-file /data/tx.raw \
  --signing-key-file /shared/shelley/genesis-utxo.skey \
  --signing-key-file /keys/funded_address.skey \
  --testnet-magic 42 \
  --out-file /data/tx.signed; then
    echo "[DEBUG] CRITICAL ERROR: Failed to sign main transaction. Aborting."
    exit 1
fi
echo "[LOG] Main funding transaction signed at /data/tx.signed."

echo "[LOG] Displaying signed main transaction details (first few lines):"
head -n 5 /data/tx.signed # Display only a few lines to avoid cluttering logs

echo "[LOG] Submitting the main funding transaction..."
if ! cardano-cli latest transaction submit \
  --tx-file /data/tx.signed \
  --testnet-magic 42; then
    echo "[DEBUG] CRITICAL ERROR: Failed to submit main transaction. Aborting."
    # Add more detailed error querying here if possible, e.g. query UTXO at $tx_in1
    exit 1
fi
echo "[LOG] Main funding transaction submitted."

echo "[LOG] Waiting 20 seconds for the main transaction to process..."
sleep 20

echo "[LOG] Verifying main transaction processing and querying UTXOs at $new_address"
cardano-cli latest query utxo --testnet-magic 42 --address "$new_address"

echo "[LOG] Saving FUNDED_ADDRESS to /shared/FUNDED_ADDRESS: $new_address"
echo "$new_address" > /shared/FUNDED_ADDRESS
echo "Created /shared/FUNDED_ADDRESS with value: $new_address"

# Registered nodes UTXOs - now query each unique address
echo "[LOG] Saving UTXO details for registered nodes (these will be empty until batch funding completes for each node)..."
for i in {1..300}; do
    node_unique_address="${registered_node_payment_addresses[$((i-1))]}"
    # echo "[LOG] Querying UTXO for registered-$i at address $node_unique_address..." # Too verbose for 300 nodes
    node_utxo=$(cardano-cli latest query utxo --testnet-magic 42 --address "$node_unique_address" 2>/dev/null | /busybox awk 'NR==3 {print $1 "#" $2}') 

    if [ -z "$node_utxo" ]; then
        # This is expected before batch funding for that node completes.
        # echo "Info: No UTXO found yet for registered-$i at address $node_unique_address." 
        echo "" > "/shared/registered-${i}.utxo"
    else
        echo "$node_utxo" > "/shared/registered-${i}.utxo"
        # echo "Saved UTXO for registered-$i: $node_utxo to /shared/registered-${i}.utxo" # Too verbose
    fi
done
echo "[LOG] Finished creating (potentially empty) UTXO files for registered nodes."

echo "[LOG] Querying and saving the first UTXO from $new_address to /shared/genesis.utxo for partner chain use..."
cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}" | /busybox awk 'NR>2 { print $1 "#" $2; exit }' > /shared/genesis.utxo
if [ -s "/shared/genesis.utxo" ]; then
    echo "[LOG] Successfully created /shared/genesis.utxo: $(cat /shared/genesis.utxo)"
    cp /shared/genesis.utxo /runtime-values/genesis.utxo
else
    echo "[LOG] ERROR: Failed to create or find UTXO for /shared/genesis.utxo from $new_address post main transaction."
    echo "[LOG] Full UTXO query output for $new_address:"
    cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}"
fi
# --- End Main Transaction Build ---

# --- Batch Funding Logic (already modified for dynamic fees) ---
echo "[LOG] Querying protocol parameters for batch funding (if not already done)..." # This line can be kept or removed if protocol_params_file definition is consolidated
protocol_params_file="/data/protocol.json" # Re-affirm or ensure it's consistently defined
if [ ! -f "$protocol_params_file" ]; then
    if ! cardano-cli latest query protocol-parameters --testnet-magic 42 --out-file "$protocol_params_file"; then
        echo "[DEBUG] CRITICAL ERROR: Failed to extract protocol parameters. Batch funding will use a fallback static fee."
    fi
fi

batch_size=10
num_batches=$(( (300 + batch_size - 1) / batch_size )) # Calculate number of batches (ceiling division)
echo "[LOG] Starting batch funding for 300 registered nodes... Batch size: $batch_size, Num batches: $num_batches"

current_funding_utxo="" # Will be populated from $new_address

# Try to find the largest lovelace UTXO at $new_address to start batch funding
echo "[DEBUG] Attempting to find the largest UTXO at $new_address for initial batch funding..."
largest_utxo_details=$(cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}" |
    /busybox awk 'NR>2 {print $1, $2, $3}' | # Print hash, ix, and amount (lovelace is $3)
    /busybox sort -k3 -n -r | # Sort by amount (numeric, reverse) 
    /busybox head -n 1) # Take the top one

if [ -n "$largest_utxo_details" ]; then
    selected_hash=$(echo "$largest_utxo_details" | /busybox awk '{print $1}')
    selected_ix=$(echo "$largest_utxo_details" | /busybox awk '{print $2}')
    selected_amount=$(echo "$largest_utxo_details" | /busybox awk '{print $3}')
    current_funding_utxo="${selected_hash}#${selected_ix}"
    echo "[DEBUG] Initial funding UTXO for batches selected: $current_funding_utxo with amount $selected_amount lovelace"
else
    echo "[DEBUG] CRITICAL ERROR: No UTXOs found at $new_address to start batch funding. This shouldn't happen if main transaction succeeded."
    # Fallback to genesis.utxo if it exists, though it might be too small
    if [ -s "/shared/genesis.utxo" ]; then
        current_funding_utxo=$(cat /shared/genesis.utxo)
        echo "[DEBUG] CRITICAL FALLBACK: Using /shared/genesis.utxo ($current_funding_utxo) for batch funding."
    else
        echo "[DEBUG] CRITICAL ERROR: Fallback /shared/genesis.utxo also not available. Cannot start batch funding."
        # Consider exiting if this is fatal: exit 1
    fi
fi

# Amount to send to each registered node in the batch
amount_per_registered_node=1000000000 # Same as defined earlier: tx_out${i}_registered
echo "[DEBUG] Amount per registered node for batches: $amount_per_registered_node"

for batch_num in $(seq 1 $num_batches); do
    echo "[DEBUG] Processing Batch $batch_num of $num_batches..."
    start_index=$(( (batch_num - 1) * batch_size ))
    end_index=$(( start_index + batch_size - 1 ))
    if [ $end_index -ge 300 ]; then
        end_index=299 # Max index is 299 for 300 nodes (0-299)
    fi

    echo "[DEBUG] Batch $batch_num: Nodes from index $start_index to $end_index."

    if [ -z "$current_funding_utxo" ]; then
        echo "[DEBUG] Batch $batch_num: No funding UTXO available from previous step. Attempting to re-query robustly..."
        largest_utxo_details_loop=$(cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}" |
            /busybox awk 'NR>2 {print $1, $2, $3}' |
            /busybox sort -k3 -n -r |
            /busybox head -n 1)
        if [ -n "$largest_utxo_details_loop" ]; then
            selected_hash_loop=$(echo "$largest_utxo_details_loop" | /busybox awk '{print $1}')
            selected_ix_loop=$(echo "$largest_utxo_details_loop" | /busybox awk '{print $2}')
            selected_amount_loop=$(echo "$largest_utxo_details_loop" | /busybox awk '{print $3}')
            current_funding_utxo="${selected_hash_loop}#${selected_ix_loop}"
            echo "[DEBUG] Batch $batch_num: Found new funding UTXO via robust query: $current_funding_utxo with amount $selected_amount_loop lovelace"
        else
            echo "[DEBUG] Batch $batch_num: CRITICAL ERROR - Still no funding UTXO after robust re-query. Aborting further batches."
            break # Exit the batch loop
        fi
    fi
    
    current_tx_in="$current_funding_utxo"
    echo "[DEBUG] Batch $batch_num: Using input UTXO: $current_tx_in"
    
    # Query the input UTXO to get its amount
    tx_in_detail=$(cardano-cli latest query utxo --testnet-magic 42 --tx-in "$current_tx_in" --out-file /dev/stdout | /busybox grep lovelace | /busybox head -n 1 | /busybox awk '{print $NF}')
    if ! [[ "$tx_in_detail" =~ ^[0-9]+$ ]]; then # Check if it's a number
        echo "[DEBUG] Error: Could not determine amount for input UTXO $current_tx_in. Trying to query all UTXOs at $new_address instead."
        # Fallback: try to get any UTXO's amount if specific one fails
        tx_in_detail_fallback=$(cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}" | /busybox grep lovelace | /busybox head -n1 | /busybox awk '{print $NF}')
        if ! [[ "$tx_in_detail_fallback" =~ ^[0-9]+$ ]]; then
             echo "[DEBUG] CRITICAL ERROR: Failed to determine input amount for batch $batch_num from $new_address. Aborting."
             current_funding_utxo="" # Reset to attempt re-query in next iteration or fail
             break
        fi
        tx_in_detail=$tx_in_detail_fallback
         echo "[DEBUG] Batch $batch_num: Determined input UTXO amount (fallback): $tx_in_detail"
    else
        echo "[DEBUG] Batch $batch_num: Input UTXO amount: $tx_in_detail"
    fi
    current_tx_in_amount=$tx_in_detail

    batch_tx_out_params=()
    batch_total_output=0

    nodes_in_this_batch=0
    for i in $(seq $start_index $end_index); do
        node_unique_address="${registered_node_payment_addresses[$i]}"
        batch_tx_out_params+=(--tx-out "$node_unique_address+$amount_per_registered_node")
        batch_total_output=$((batch_total_output + amount_per_registered_node))
        nodes_in_this_batch=$((nodes_in_this_batch + 1))
    done

    echo "[DEBUG] Batch $batch_num: Processing $nodes_in_this_batch nodes. Total output for nodes: $batch_total_output"

    if [ $nodes_in_this_batch -eq 0 ]; then
        echo "[DEBUG] Batch $batch_num: No nodes in this batch. Skipping."
        continue
    fi

    # Dynamic Fee Calculation
    echo "[DEBUG] Batch $batch_num: Building DUMMY transaction for fee calculation..."
    dummy_change_placeholder=1000000 # A small positive placeholder for change output
    temp_batch_tx_out_params=("${batch_tx_out_params[@]}") # Copy existing outputs for nodes
    temp_batch_tx_out_params+=(--tx-out "$new_address+$dummy_change_placeholder") # Add placeholder change for $new_address

    dummy_tx_file="/data/tx_batch_${batch_num}_dummy.raw"
    if ! cardano-cli latest transaction build-raw \
      --tx-in "$current_tx_in" \
      "${temp_batch_tx_out_params[@]}" \
      --fee 0 \
      --out-file "$dummy_tx_file"; then
        echo "[DEBUG] Batch $batch_num: ERROR building DUMMY transaction for fee calculation. Skipping batch."
        current_funding_utxo="" 
        rm -f "$dummy_tx_file"
        continue
    fi

    num_inputs=1
    num_outputs=$((nodes_in_this_batch + 1)) # N outputs to nodes + 1 change output
    num_witnesses=1 # Signed by funded_address.skey

    echo "[DEBUG] Batch $batch_num: Calculating min fee. Inputs: $num_inputs, Outputs: $num_outputs, Witnesses: $num_witnesses, Protocol File: $protocol_params_file"
    calculated_fee=$(cardano-cli latest transaction calculate-min-fee \
        --tx-body-file "$dummy_tx_file" \
        --testnet-magic 42 \
        --protocol-params-file "$protocol_params_file" \
        --tx-in-count "$num_inputs" \
        --tx-out-count "$num_outputs" \
        --witness-count "$num_witnesses" | /busybox awk '{print $1}')

    if ! [[ "$calculated_fee" =~ ^[0-9]+$ ]] || [ -z "$calculated_fee" ]; then
        echo "[DEBUG] Batch $batch_num: ERROR calculating dynamic fee (Raw output: '$calculated_fee'). Using fallback static fee 250000."
        batch_fee=250000 
    else
        batch_fee=$((calculated_fee + 30000)) # Add a 30000 lovelace buffer
        echo "[DEBUG] Batch $batch_num: Calculated Min Fee: $calculated_fee, Using Fee with Buffer: $batch_fee"
    fi
    rm -f "$dummy_tx_file" # Clean up dummy transaction file

    batch_change=$((current_tx_in_amount - batch_total_output - batch_fee))
    echo "[DEBUG] Batch $batch_num: Fee=$batch_fee, Change=$batch_change (Input: $current_tx_in_amount, NodesOutput: $batch_total_output)"

    if [ "$batch_change" -lt 1000000 ]; then # Check if change is less than 1 ADA (or some other reasonable minimum)
        echo "[DEBUG] Batch $batch_num: ERROR - Not enough funds or change too small ($batch_change). Required for nodes+fee: $((batch_total_output + batch_fee)), Available from input: $current_tx_in_amount. Skipping batch."
        current_funding_utxo="" 
        continue
    fi

    # Finalize transaction parameters with calculated fee and change
    final_batch_tx_out_params=("${batch_tx_out_params[@]}") # Outputs to registered nodes
    final_batch_tx_out_params+=(--tx-out "$new_address+$batch_change") # Actual change output to $new_address

    echo "[DEBUG] Batch $batch_num: Building final transaction..."
    if ! cardano-cli latest transaction build-raw \
      --tx-in "$current_tx_in" \
      "${final_batch_tx_out_params[@]}" \
      --fee "$batch_fee" \
      --out-file "/data/tx_batch_${batch_num}.raw"; then
      echo "[DEBUG] Batch $batch_num: CRITICAL ERROR building FINAL transaction. Skipping batch."
      current_funding_utxo=""
      rm -f "/data/tx_batch_${batch_num}.raw" # Clean up potentially incomplete raw file
      continue
    fi

    echo "[DEBUG] Batch $batch_num: Signing transaction..."
    if ! cardano-cli latest transaction sign \
      --tx-body-file "/data/tx_batch_${batch_num}.raw" \
      --signing-key-file /keys/funded_address.skey \
      --testnet-magic 42 \
      --out-file "/data/tx_batch_${batch_num}.signed"; then
      echo "[DEBUG] Batch $batch_num: CRITICAL ERROR signing transaction. Skipping batch."
      current_funding_utxo=""
      rm -f "/data/tx_batch_${batch_num}.raw" "/data/tx_batch_${batch_num}.signed"
      continue
    fi

    echo "[DEBUG] Batch $batch_num: Submitting transaction..."
    if ! cardano-cli latest transaction submit \
      --tx-file "/data/tx_batch_${batch_num}.signed" \
      --testnet-magic 42; then
      echo "[DEBUG] Batch $batch_num: ERROR submitting transaction. The input UTXO $current_tx_in might still be available. Will re-evaluate for next batch."
      # Do not clear current_funding_utxo here, let the outer logic re-evaluate using largest UTXO
      # The transaction failed, so the input UTXO was not consumed.
    else
      echo "[DEBUG] Batch $batch_num: Successfully submitted."
    fi
    
    echo "[DEBUG] Batch $batch_num: Waiting 15 seconds for processing..."
    sleep 15

    echo "[DEBUG] Batch $batch_num: Attempting to find largest available UTXO at $new_address for the next batch..."
    largest_utxo_details=$(cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}" |
        /busybox awk 'NR>2 {print $1, $2, $3}' | # Print hash, ix, and amount (lovelace is $3)
        /busybox sort -k3 -n -r | # Sort by amount (numeric, reverse)
        /busybox head -n 1) # Take the top one

    if [ -n "$largest_utxo_details" ]; then
        selected_hash=$(echo "$largest_utxo_details" | /busybox awk '{print $1}')
        selected_ix=$(echo "$largest_utxo_details" | /busybox awk '{print $2}')
        selected_amount=$(echo "$largest_utxo_details" | /busybox awk '{print $3}')
        current_funding_utxo="${selected_hash}#${selected_ix}"
        echo "[DEBUG] Batch $batch_num: New funding UTXO for next batch: $current_funding_utxo with amount $selected_amount lovelace"
    else
        echo "[DEBUG] Batch $batch_num: CRITICAL ERROR - Could not find any UTXO at $new_address after batch $batch_num. Aborting further batches."
        current_funding_utxo="" # Ensure it's empty so next iteration also tries to re-query robustly or fails
        break # Exit the batch loop as we can't proceed
    fi
done
echo "[LOG] Batch funding for registered nodes complete."

echo "[LOG] Generating Mainchain Cold Keys for Registered Nodes..."
for i in {1..300}; do
    NODE_KEYS_DIR="/shared/node-keys/registered-${i}/keys"
    mkdir -p "$NODE_KEYS_DIR"
    echo "[LOG] Generating cold keys for registered-$i in $NODE_KEYS_DIR..."
    cardano-cli node key-gen \
        --cold-verification-key-file "${NODE_KEYS_DIR}/cold.vkey" \
        --cold-signing-key-file "${NODE_KEYS_DIR}/cold.skey" \
        --operational-certificate-issue-counter-file "${NODE_KEYS_DIR}/cold.counter"
    if [ $? -eq 0 ]; then
        echo "[LOG] Successfully generated cold keys for registered-$i."
    else
        echo "Error generating cold keys for registered-$i!"
        # Optionally exit here if this is critical
    fi
done
echo "[LOG] Finished generating mainchain cold keys."

echo "[LOG] Creating /shared/cardano.ready signal file."
touch /shared/cardano.ready

echo "[LOG] Cardano node entrypoint script finished. Waiting for node process to terminate (if it does)."
wait
