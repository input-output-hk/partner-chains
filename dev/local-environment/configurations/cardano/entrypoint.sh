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

# --- START OF NEW BATCH FUNDING LOGIC ---
echo "[LOG] Main transaction submitted. Determining initial UTXO for batch funding registered nodes."

# Determine the TxId of the main transaction
# /data/tx.signed was the file submitted for the main transaction.
main_tx_id=$(cardano-cli latest transaction txid --tx-file /data/tx.signed)
if [ -z "$main_tx_id" ]; then
    echo "[DEBUG] CRITICAL ERROR: Failed to get TxId from main signed transaction /data/tx.signed. Aborting batch funding."
    exit 1
fi
echo "[LOG] Main transaction ID for funding: $main_tx_id"

# num_main_tx_outputs_before_change was calculated at line 196 when building the main transaction.
# It represents the count of outputs before the change output was added.
# So, the 0-indexed TxIx of the change output is equal to num_main_tx_outputs_before_change.
initial_funding_utxo_tx_ix="$num_main_tx_outputs_before_change"
current_batch_input_utxo="${main_tx_id}#${initial_funding_utxo_tx_ix}"

echo "[LOG] Batch funding will start using main transaction's change output: $current_batch_input_utxo"
echo "[LOG] Querying all UTXOs at $new_address for verification after main transaction:"
cardano-cli latest query utxo --testnet-magic 42 --address "$new_address" --out-file /data/utxos_at_new_address_after_main.json
if [ -s /data/utxos_at_new_address_after_main.json ]; then
    echo "Full list of UTXOs at $new_address (content of /data/utxos_at_new_address_after_main.json):"
    cat /data/utxos_at_new_address_after_main.json
else
    echo "[WARN] Could not retrieve full list of UTXOs at $new_address or file is empty."
fi


# Batch funding loop for registered_node_payment_addresses
num_registered_nodes=${#registered_node_payment_addresses[@]} # Get actual count from array
batch_size=10 # Fund 10 nodes per batch
num_batches=$(( (num_registered_nodes + batch_size - 1) / batch_size )) # Ceiling division

# Ensure protocol_params_file is defined (it's set around line 198: protocol_params_file="/data/protocol.json")
if [ ! -f "$protocol_params_file" ]; then
    echo "[DEBUG] CRITICAL ERROR: Protocol parameters file $protocol_params_file not found before batch funding. Re-querying..."
    if ! cardano-cli latest query protocol-parameters --testnet-magic 42 --out-file "$protocol_params_file"; then
        echo "[DEBUG] CRITICAL ERROR: Failed to re-query protocol parameters. Aborting."
        exit 1
    fi
fi


echo "[LOG] Starting batch funding for $num_registered_nodes registered nodes in $num_batches batches of up to $batch_size nodes each."

for batch_num in $(seq 1 "$num_batches"); do
    echo "[LOG] Processing Batch $batch_num of $num_batches..."
    start_node_array_idx=$(( (batch_num - 1) * batch_size )) # 0-indexed for array
    end_node_array_idx=$(( start_node_array_idx + batch_size - 1 ))
    if [ "$end_node_array_idx" -ge "$num_registered_nodes" ]; then
        end_node_array_idx=$((num_registered_nodes - 1))
    fi

    node_idx_human_start=$((start_node_array_idx + 1))
    node_idx_human_end=$((end_node_array_idx + 1))
    echo "[LOG] Batch $batch_num: Funding nodes $node_idx_human_start to $node_idx_human_end."

    batch_tx_out_params_array=()
    batch_total_output_lovelace=0
    actual_nodes_in_this_batch=0

    for (( current_array_idx=start_node_array_idx; current_array_idx<=end_node_array_idx; current_array_idx++ )); do
        node_payment_address="${registered_node_payment_addresses[$current_array_idx]}"
        node_funding_amount=1000000 # 1 ADA
        batch_tx_out_params_array+=(--tx-out "$node_payment_address+$node_funding_amount")
        batch_total_output_lovelace=$((batch_total_output_lovelace + node_funding_amount))
        actual_nodes_in_this_batch=$((actual_nodes_in_this_batch + 1))
    done

    if [ "$actual_nodes_in_this_batch" -eq 0 ]; then
        echo "[LOG] Batch $batch_num: No nodes to fund in this batch. Skipping."
        continue
    fi

    echo "[LOG] Batch $batch_num: Building DUMMY transaction for fee calculation. Outputs to fund: $actual_nodes_in_this_batch"
    dummy_batch_tx_file="/data/tx_batch_${batch_num}_dummy.raw"
    dummy_batch_change_placeholder=1000000 # Min 1 ADA for dummy change

    temp_batch_tx_out_params_array=("${batch_tx_out_params_array[@]}")
    temp_batch_tx_out_params_array+=(--tx-out "$new_address+$dummy_batch_change_placeholder") # Dummy change to new_address

    if ! cardano-cli latest transaction build-raw \
      --tx-in "$current_batch_input_utxo" \
      "${temp_batch_tx_out_params_array[@]}" \
      --fee 0 \
      --out-file "$dummy_batch_tx_file"; then
        echo "[DEBUG] Batch $batch_num: ERROR building DUMMY transaction for fee calculation. Input UTXO: $current_batch_input_utxo. Using fallback fee."
        cat "$dummy_batch_tx_file" 2>/dev/null || echo "Dummy file $dummy_batch_tx_file not created or unreadable."
        batch_tx_fee=200000 # Fallback
    else
        batch_tx_num_inputs=1
        batch_tx_num_outputs=$((actual_nodes_in_this_batch + 1)) # N outputs to nodes + 1 change output
        batch_tx_num_witnesses=1 # Signed by funded_address.skey

        calculated_batch_fee=$(cardano-cli latest transaction calculate-min-fee \
            --tx-body-file "$dummy_batch_tx_file" \
            --testnet-magic 42 \
            --protocol-params-file "$protocol_params_file" \
            --tx-in-count "$batch_tx_num_inputs" \
            --tx-out-count "$batch_tx_num_outputs" \
            --witness-count "$batch_tx_num_witnesses" | /busybox awk '{print $1}')

        if ! [[ "$calculated_batch_fee" =~ ^[0-9]+$ ]] || [ -z "$calculated_batch_fee" ]; then
            echo "[DEBUG] Batch $batch_num: ERROR calculating dynamic fee (Raw: '$calculated_batch_fee'). Using fallback 200000."
            batch_tx_fee=200000
        else
            batch_tx_fee=$((calculated_batch_fee + 1000)) # Add a 1000 lovelace buffer
            echo "[LOG] Batch $batch_num: Calculated Min Fee: $calculated_batch_fee, Using Fee with Buffer: $batch_tx_fee"
        fi
        rm -f "$dummy_batch_tx_file"
    fi

    echo "[LOG] Batch $batch_num: Total output to registered nodes: $batch_total_output_lovelace, Calculated Fee: $batch_tx_fee"

    echo "[LOG] Batch $batch_num: Querying current value of input UTXO $current_batch_input_utxo"
    input_utxo_details_file="/data/input_utxo_details_batch_${batch_num}.json"
    # Retry mechanism for querying input UTXO in case of slight delay
    utxo_queried_successfully=false
    for query_attempt in {1..3}; do
        if cardano-cli latest query utxo --testnet-magic 42 --tx-in "$current_batch_input_utxo" --out-file "$input_utxo_details_file"; then
            if [ -s "$input_utxo_details_file" ]; then
                utxo_queried_successfully=true
                break
            fi
        fi
        echo "[WARN] Batch $batch_num: Attempt $query_attempt to query input UTXO $current_batch_input_utxo failed or returned empty. Retrying in 3s..."
        sleep 3
    done

    if [ "$utxo_queried_successfully" = false ]; then
        echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: Failed to query input UTXO $current_batch_input_utxo after multiple attempts. It might be spent or invalid."
        echo "Attempting to find a new UTXO at $new_address as a fallback..."
        new_potential_utxo=$(cardano-cli latest query utxo --address "$new_address" --testnet-magic 42 | grep lovelace | sort -k3 -nr | head -n 1 | awk '{print $1"#"$2}')
        if [ -n "$new_potential_utxo" ] && [ "$new_potential_utxo" != "$current_batch_input_utxo" ]; then
            echo "[LOG] Found alternative UTXO: $new_potential_utxo. Retrying with this one for batch $batch_num."
            current_batch_input_utxo="$new_potential_utxo"
            if ! cardano-cli latest query utxo --testnet-magic 42 --tx-in "$current_batch_input_utxo" --out-file "$input_utxo_details_file"; then
                 echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: Fallback query for new UTXO $current_batch_input_utxo also failed. Aborting."
                 exit 1
            fi
        else
            echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: No alternative UTXO found or it's the same problematic one. Aborting."
            exit 1
        fi
    fi
    
    if ! jq_output=$(/busybox jq -r '.value.lovelace' "$input_utxo_details_file" 2>/dev/null); then
        echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: Failed to parse lovelace from UTXO $current_batch_input_utxo details."
        echo "Contents of $input_utxo_details_file (if any):"
        cat "$input_utxo_details_file"
        exit 1
    fi
    current_batch_input_utxo_amount=$jq_output
    rm -f "$input_utxo_details_file"

    if ! [[ "$current_batch_input_utxo_amount" =~ ^[0-9]+$ ]]; then
        echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: Parsed lovelace amount '$current_batch_input_utxo_amount' for $current_batch_input_utxo is not a number. Aborting."
        exit 1
    fi
    echo "[LOG] Batch $batch_num: Input UTXO $current_batch_input_utxo has amount $current_batch_input_utxo_amount lovelace."

    batch_tx_change=$((current_batch_input_utxo_amount - batch_total_output_lovelace - batch_tx_fee))
    echo "[LOG] Batch $batch_num: Change calculated: $batch_tx_change (Input: $current_batch_input_utxo_amount - Outputs: $batch_total_output_lovelace - Fee: $batch_tx_fee)"

    final_batch_tx_out_params_array=()
    if [ "$batch_tx_change" -ge 1000000 ]; then 
        final_batch_tx_out_params_array+=(--tx-out "$new_address+$batch_tx_change") 
        final_batch_tx_out_params_array+=("${batch_tx_out_params_array[@]}") 
    elif [ "$batch_tx_change" -ge 0 ]; then 
        echo "[WARN] Batch $batch_num: Change is very small or zero ($batch_tx_change lovelace). It will be added to the fee. No separate change UTXO will be created."
        batch_tx_fee=$((batch_tx_fee + batch_tx_change)) 
        batch_tx_change=0 
        final_batch_tx_out_params_array=("${batch_tx_out_params_array[@]}") 
    else
        echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: Negative change ($batch_tx_change). Input $current_batch_input_utxo_amount, outputs $batch_total_output_lovelace, fee $batch_tx_fee. Aborting."
        exit 1
    fi

    batch_tx_raw_file="/data/tx_batch_${batch_num}.raw"
    batch_tx_signed_file="/data/tx_batch_${batch_num}.signed"

    echo "[LOG] Batch $batch_num: Building FINAL transaction raw file..."
    if ! cardano-cli latest transaction build-raw \
      --tx-in "$current_batch_input_utxo" \
      "${final_batch_tx_out_params_array[@]}" \
      --fee "$batch_tx_fee" \
      --out-file "$batch_tx_raw_file"; then
        echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: Failed to build FINAL transaction. Aborting."
        exit 1
    fi

    echo "[LOG] Batch $batch_num: Signing transaction..."
    if ! cardano-cli latest transaction sign \
      --tx-body-file "$batch_tx_raw_file" \
      --signing-key-file /keys/funded_address.skey \
      --testnet-magic 42 \
      --out-file "$batch_tx_signed_file"; then
        echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: Failed to sign transaction. Aborting."
        exit 1
    fi

    batch_tx_id=$(cardano-cli latest transaction txid --tx-file "$batch_tx_signed_file")
    if [ -z "$batch_tx_id" ]; then
        echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: Failed to get TxId from signed transaction. Aborting."
        exit 1
    fi
    echo "[LOG] Batch $batch_num: Transaction ID is $batch_tx_id"

    echo "[LOG] Batch $batch_num: Submitting transaction..."
    # Retry mechanism for submission
    submitted_successfully=false
    for submit_attempt in {1..3}; do
        if cardano-cli latest transaction submit \
          --tx-file "$batch_tx_signed_file" \
          --testnet-magic 42; then
            submitted_successfully=true
            break
        fi
        echo "[WARN] Batch $batch_num: Attempt $submit_attempt to submit transaction $batch_tx_id failed. Retrying in 5s..."
        sleep 5
    done

    if [ "$submitted_successfully" = false ]; then
        echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: Failed to submit transaction $batch_tx_id after multiple attempts. Aborting."
        echo "Querying input UTXO $current_batch_input_utxo to check if it was spent:"
        cardano-cli latest query utxo --testnet-magic 42 --tx-in "$current_batch_input_utxo" --out-file /dev/stdout || echo "Query failed or UTXO spent."
        exit 1
    fi
    echo "[LOG] Batch $batch_num: Transaction $batch_tx_id submitted."
    
    if [ "$batch_tx_change" -ge 1000000 ]; then 
        current_batch_input_utxo="${batch_tx_id}#0" 
        echo "[LOG] Batch $batch_num: Next input UTXO for funding will be the change from this batch: $current_batch_input_utxo"
    elif [ "$batch_num" -lt "$num_batches" ]; then 
        echo "[WARN] Batch $batch_num: No usable change output created. Attempting to find a new UTXO at $new_address for the next batch."
        sleep 5 
        new_input_utxo_candidate=$(cardano-cli latest query utxo --address "$new_address" --testnet-magic 42 | grep lovelace | sort -k3 -nr | head -n 1 | awk '{print $1"#"$2}')
        if [ -n "$new_input_utxo_candidate" ]; then
            current_batch_input_utxo="$new_input_utxo_candidate"
            echo "[LOG] Batch $batch_num: Found new input UTXO for next batch: $current_batch_input_utxo"
        else
            echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: No change output from this batch and no other UTXOs found at $new_address. Cannot continue funding. Aborting."
            exit 1
        fi
    fi
    
    rm -f "$batch_tx_raw_file" "$batch_tx_signed_file"

    if [ "$batch_num" -lt "$num_batches" ]; then
        echo "[LOG] Batch $batch_num: Waiting 15 seconds for transaction to process before next batch..."
        sleep 15
    else
        echo "[LOG] Batch $batch_num: Final batch processed."
    fi
done

echo "[LOG] Completed all $num_batches batch funding transactions for registered nodes."
# --- END OF NEW BATCH FUNDING LOGIC ---

echo "[LOG] Saving FUNDED_ADDRESS to /shared/FUNDED_ADDRESS: $new_address"
echo "$new_address" > /shared/FUNDED_ADDRESS
echo "Created /shared/FUNDED_ADDRESS with value: $new_address"

# Registered nodes UTXOs - now query each unique address
echo "[LOG] Finalizing UTXO files for registered nodes after all batch funding..."
for i in {1..300}; do
    node_unique_address="${registered_node_payment_addresses[$((i-1))]}"
    # It's critical that the UTXO exists now. Add some retries just in case of chain lag.
    final_utxo_found=false
    for attempt in {1..5}; do # Try up to 5 times
        echo "[LOG] Querying final UTXO for registered-$i at $node_unique_address (Attempt $attempt)..."
        
        raw_cli_output_file="/tmp/raw_cli_output_registered_${i}_attempt_${attempt}.txt"
        echo "[DEBUG] Attempting to run for $node_unique_address (Attempt $attempt): cardano-cli latest query utxo --testnet-magic 42 --address \"$node_unique_address\" --out-file /dev/stdout"
        cardano-cli latest query utxo --testnet-magic 42 --address "$node_unique_address" --out-file /dev/stdout > "$raw_cli_output_file" 2>&1

        echo "[DEBUG] Raw output from cardano-cli for $node_unique_address (Attempt $attempt) captured in $raw_cli_output_file:"
        cat "$raw_cli_output_file"

        node_utxo_final=$(cat "$raw_cli_output_file" | /busybox awk 'NR>2 {print $1 "#" $2; exit}')
        echo "[DEBUG] Parsed node_utxo_final by awk: [$node_utxo_final]"
        
        if [ -n "$node_utxo_final" ]; then
            if [[ "$node_utxo_final" =~ ^[a-f0-9]{64}#[0-9]+$ ]]; then
                echo "$node_utxo_final" > "/shared/registered-${i}.utxo"
                echo "[LOG] Successfully updated /shared/registered-${i}.utxo with: $node_utxo_final"
                final_utxo_found=true
                break
            else
                echo "[WARN] Attempt $attempt: For registered-$i at $node_unique_address, query output [$node_utxo_final] does not look like TxHash#TxIx. Retrying..."
                node_utxo_final="" 
            fi
        else
            echo "[WARN] Attempt $attempt: No UTXO found yet for registered-$i at $node_unique_address. Sleeping 5s..."
            sleep 5
        fi
    done

    if [ "$final_utxo_found" = false ]; then
        echo "[ERROR] CRITICAL: Failed to find UTXO for registered-$i at $node_unique_address after multiple attempts. /shared/registered-${i}.utxo will be empty. This will likely cause registration to fail for this node."
        echo "" > "/shared/registered-${i}.utxo"
    fi
done
echo "[LOG] Finished finalizing UTXO files for all registered nodes."

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
    fi
done
echo "[LOG] Finished generating mainchain cold keys."

echo "[LOG] Creating /shared/cardano.ready signal file."
touch /shared/cardano.ready

echo "[LOG] Cardano node entrypoint script finished. Waiting for node process to terminate (if it does)."
wait
