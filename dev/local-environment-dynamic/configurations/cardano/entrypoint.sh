#! /bin/bash
set -e

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

echo "[LOG] Waiting 10 seconds to ensure node.socket is ready..."
sleep 10

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
registered_node_payment_addresses=() # Array to store payment addresses

# --- New: Generate KES, VRF, and Stake Keypairs for Registered Nodes FIRST ---
echo "[LOG] Generating KES, VRF, and Stake Keypairs for $NUM_REGISTERED_NODES_TO_PROCESS Registered Nodes..."
for i in $(seq 1 $NUM_REGISTERED_NODES_TO_PROCESS); do
    NODE_SPECIFIC_KEYS_DIR="/shared/node-keys/registered-${i}/keys"
    mkdir -p "$NODE_SPECIFIC_KEYS_DIR"

    echo "[LOG] Generating KES keys for registered-$i in $NODE_SPECIFIC_KEYS_DIR..."
    cardano-cli node key-gen-KES \
        --verification-key-file "${NODE_SPECIFIC_KEYS_DIR}/kes.vkey" \
        --signing-key-file "${NODE_SPECIFIC_KEYS_DIR}/kes.skey"
    if [ $? -ne 0 ]; then
        echo "Error generating KES keys for registered-$i!"
    fi

    echo "[LOG] Generating VRF keys for registered-$i in $NODE_SPECIFIC_KEYS_DIR..."
    cardano-cli node key-gen-VRF \
        --verification-key-file "${NODE_SPECIFIC_KEYS_DIR}/vrf.vkey" \
        --signing-key-file "${NODE_SPECIFIC_KEYS_DIR}/vrf.skey"
    if [ $? -ne 0 ]; then
        echo "Error generating VRF keys for registered-$i!"
    fi

    echo "[LOG] Generating Stake keys for registered-$i in $NODE_SPECIFIC_KEYS_DIR..."
    cardano-cli shelley stake-address key-gen \
        --verification-key-file "${NODE_SPECIFIC_KEYS_DIR}/stake.vkey" \
        --signing-key-file "${NODE_SPECIFIC_KEYS_DIR}/stake.skey"
    if [ $? -ne 0 ]; then
        echo "Error generating Stake keys for registered-$i!"
    fi
done
echo "[LOG] Completed generation of KES, VRF, and Stake keypairs for $NUM_REGISTERED_NODES_TO_PROCESS registered nodes."
# --- End of Key Generation ---

# --- New: Generate Stake Addresses for Registered Nodes (NOW with stake.vkey available) ---
echo "[LOG] Generating Stake Addresses for $NUM_REGISTERED_NODES_TO_PROCESS Registered Nodes..."
registered_node_stake_addresses=() # Array to store stake addresses
for i in $(seq 1 $NUM_REGISTERED_NODES_TO_PROCESS); do
    NODE_SPECIFIC_KEYS_DIR="/shared/node-keys/registered-${i}/keys"
    echo "[LOG] Generating stake address for registered-$i in $NODE_SPECIFIC_KEYS_DIR..."
    node_stake_address=$(cardano-cli shelley stake-address build \
        --stake-verification-key-file "${NODE_SPECIFIC_KEYS_DIR}/stake.vkey" \
        --testnet-magic 42)

    if [ -z "$node_stake_address" ]; then
        echo "Error building stake address for registered-$i!"
    else
        registered_node_stake_addresses+=("$node_stake_address")
        echo "[LOG] Generated stake address for registered-$i: $node_stake_address"
    fi
done
echo "[LOG] Completed generation of stake addresses for $NUM_REGISTERED_NODES_TO_PROCESS registered nodes."
# --- End of Stake Address Generation ---

# --- New: Generate Payment Keys and Addresses for Registered Nodes ---
echo "[LOG] Generating Payment Keys and Addresses for $NUM_REGISTERED_NODES_TO_PROCESS Registered Nodes..."
for i in $(seq 1 $NUM_REGISTERED_NODES_TO_PROCESS); do
    NODE_SPECIFIC_KEYS_DIR="/shared/node-keys/registered-${i}/keys"
    
    echo "[LOG] Generating payment keys for registered-$i in $NODE_SPECIFIC_KEYS_DIR..."
    cardano-cli address key-gen \
        --verification-key-file "${NODE_SPECIFIC_KEYS_DIR}/payment.vkey" \
        --signing-key-file "${NODE_SPECIFIC_KEYS_DIR}/payment.skey"
    if [ $? -ne 0 ]; then 
        echo "Error generating payment keys for registered-$i!"
        continue
    fi
    
    node_payment_address=$(cardano-cli address build \
        --payment-verification-key-file "${NODE_SPECIFIC_KEYS_DIR}/payment.vkey" \
        --stake-verification-key-file "${NODE_SPECIFIC_KEYS_DIR}/stake.vkey" \
        --testnet-magic 42)
    if [ -z "$node_payment_address" ]; then 
        echo "Error building payment address for registered-$i!"
        continue
    fi
    
    registered_node_payment_addresses+=("$node_payment_address")
    echo "[LOG] Generated payment address for registered-$i: $node_payment_address"
done
echo "[LOG] Completed generation of payment keys and addresses for $NUM_REGISTERED_NODES_TO_PROCESS registered nodes."
# --- End of Payment Key Generation ---

echo "[LOG] Completed generation of payment keypairs and addresses for $NUM_REGISTERED_NODES_TO_PROCESS registered nodes."

# Debug: Verify that the arrays are populated
echo "[DEBUG] Verifying populated arrays..."
echo "[DEBUG] Number of registered payment addresses: ${#registered_node_payment_addresses[@]}"
echo "[DEBUG] Number of registered stake addresses: ${#registered_node_stake_addresses[@]}"

if [ "${#registered_node_payment_addresses[@]}" -eq 0 ]; then
    echo "[DEBUG] CRITICAL ERROR: registered_node_payment_addresses array is empty! This will cause batch funding to fail."
    exit 1
fi

for i in $(seq 0 $((${#registered_node_payment_addresses[@]} - 1))); do
    echo "[DEBUG]: registered_node_payment_addresses[$i] = ${registered_node_payment_addresses[$i]}"
done
# End Debug

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

# Fund $NUM_REGISTERED_NODES_TO_PROCESS registered nodes (These are defined but not used in the *initial* main transaction anymore)
# They are funded in batches later.
for i in $(seq 1 $NUM_REGISTERED_NODES_TO_PROCESS); do
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

echo "[LOG] Waiting 45 seconds for the main transaction to process and be confirmed..."
sleep 45

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
verification_utxos=$(cardano-cli latest query utxo --testnet-magic 42 --address "$new_address" --out-file /data/utxos_at_new_address_after_main.json 2>&1)
echo "[DEBUG] Verification query exit code: $?"
echo "[DEBUG] Verification query stderr/stdout: $verification_utxos"
if [ -s /data/utxos_at_new_address_after_main.json ]; then
    echo "[LOG] Full list of UTXOs at $new_address saved to /data/utxos_at_new_address_after_main.json"
else
    echo "[WARN] Could not retrieve full list of UTXOs at $new_address or file is empty."
fi


# Batch funding loop for registered_node_payment_addresses
num_registered_nodes=$NUM_REGISTERED_NODES_TO_PROCESS # Use the configuration variable
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

# Validate batch funding setup for large numbers
if [ "$num_registered_nodes" -gt 1000 ]; then
    echo "[WARN] Large number of registered nodes ($num_registered_nodes). This will create $num_batches batches."
    echo "[WARN] Each batch will require ~200-300k lovelace in fees (base fee + reference script overhead)."
    total_estimated_fees=$((num_batches * 250000)) # Rough estimate: 250k lovelace per batch
    echo "[WARN] Estimated total fees for all batches: ~$total_estimated_fees lovelace"
fi

# Validate batch size doesn't exceed reasonable limits
if [ "$batch_size" -gt 50 ]; then
    echo "[WARN] Batch size of $batch_size is very large. Consider reducing to improve fee estimation accuracy."
fi

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
        node_funding_amount=600000000 # 600 ADA - covers SPO registration (500 ADA pool deposit + 0.4 ADA stake deposit + generous buffer for fees)
        batch_tx_out_params_array+=(--tx-out "$node_payment_address+$node_funding_amount")
        batch_total_output_lovelace=$((batch_total_output_lovelace + node_funding_amount))
        actual_nodes_in_this_batch=$((actual_nodes_in_this_batch + 1))
    done

    if [ "$actual_nodes_in_this_batch" -eq 0 ]; then
        echo "[LOG] Batch $batch_num: No nodes to fund in this batch. Skipping."
        continue
    fi

    echo "[LOG] Batch $batch_num: Querying current value of input UTXO $current_batch_input_utxo"
    
    # First, get the actual UTXO information so we can calculate fees accurately
    utxo_queried_successfully=false
    for query_attempt in {1..5}; do # Increased attempts since we need to wait for confirmation
        echo "[LOG] Batch $batch_num: Querying address UTXOs attempt $query_attempt..."
        
        # Query all UTXOs at the address (this is the correct syntax)
        echo "[DEBUG] Batch $batch_num: Running: cardano-cli latest query utxo --testnet-magic 42 --address \"$new_address\""
        address_utxos_file="/data/address_utxos_batch_${batch_num}_attempt_${query_attempt}.json"
        
        if cardano-cli latest query utxo --testnet-magic 42 --address "$new_address" --out-file "$address_utxos_file"; then
            echo "[DEBUG] Batch $batch_num: Address query successful, file size: $(wc -c < "$address_utxos_file" 2>/dev/null || echo "unknown")"
            
            # Check if our specific UTXO exists in the results using simple text search
            if /busybox grep -q "$current_batch_input_utxo" "$address_utxos_file"; then
                echo "[LOG] Batch $batch_num: Found target UTXO $current_batch_input_utxo in output"
                
                # Extract the UTXO entry with context lines (get the full JSON object)
                utxo_context=$(/busybox grep -A 20 "$current_batch_input_utxo" "$address_utxos_file")
                
                # Extract lovelace amount from the context
                current_batch_input_utxo_amount=$(echo "$utxo_context" | /busybox grep '"lovelace":' | /busybox grep -o '[0-9]\+' | head -1)
                
                if [[ "$current_batch_input_utxo_amount" =~ ^[0-9]+$ ]] && [ "$current_batch_input_utxo_amount" -gt 0 ]; then
                    echo "[LOG] Batch $batch_num: Successfully extracted UTXO amount: $current_batch_input_utxo_amount lovelace"
                    utxo_queried_successfully=true
                    rm -f "$address_utxos_file"
                    break
                else
                    echo "[WARN] Batch $batch_num: Failed to extract valid lovelace amount from context. Got: '$current_batch_input_utxo_amount'"
                fi
            else
                echo "[WARN] Batch $batch_num: Target UTXO $current_batch_input_utxo not found in output"
            fi
            
            rm -f "$address_utxos_file"
        else
            echo "[WARN] Batch $batch_num: Address query failed on attempt $query_attempt"
        fi
        
        if [ "$utxo_queried_successfully" = false ]; then
            echo "[WARN] Batch $batch_num: Attempt $query_attempt failed. Waiting 10 seconds for transaction confirmation..."
            sleep 10
        fi
    done

    if [ "$utxo_queried_successfully" = false ]; then
        echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: Failed to find UTXO $current_batch_input_utxo after multiple address queries."
        echo "Attempting to find any large UTXO at $new_address as a fallback..."
        
        # Fallback: find the largest available UTXO
        fallback_utxos_file="/data/fallback_utxos_batch_${batch_num}.json"
        if cardano-cli latest query utxo --testnet-magic 42 --address "$new_address" --out-file "$fallback_utxos_file"; then
            echo "[DEBUG] Batch $batch_num: Fallback query successful, analyzing available UTXOs..."
            echo "[DEBUG] Batch $batch_num: Fallback output (first 10 lines):"
            echo "[DEBUG] === START OF FALLBACK OUTPUT ==="
            head -10 "$fallback_utxos_file" 2>&1 | while IFS= read -r line; do echo "[DEBUG] $line"; done || echo "[DEBUG] Failed to read file"
            echo "[DEBUG] === END OF FALLBACK OUTPUT ==="
            
            # Find largest UTXO using grep context approach (no jq)
            echo "[DEBUG] Batch $batch_num: Finding largest UTXO using text parsing..."
            largest_amount=0
            largest_utxo=""
            
            # Get all UTXO identifiers first
            all_utxos=$(/busybox grep -o '[a-f0-9]\{64\}#[0-9]\+' "$fallback_utxos_file")
            echo "[DEBUG] Batch $batch_num: Found UTXOs in fallback:"
            echo "$all_utxos" | while IFS= read -r line; do echo "[DEBUG] UTXO: $line"; done
            
            # For each UTXO, extract its lovelace amount using context
            while IFS= read -r utxo_id; do
                if [ -n "$utxo_id" ]; then
                    echo "[DEBUG] Batch $batch_num: Checking UTXO $utxo_id..."
                    utxo_context=$(/busybox grep -A 20 "$utxo_id" "$fallback_utxos_file")
                    line_amount=$(echo "$utxo_context" | /busybox grep '"lovelace":' | /busybox grep -o '[0-9]\+' | head -1)
                    
                    if [[ "$line_amount" =~ ^[0-9]+$ ]] && [ "$line_amount" -gt "$largest_amount" ]; then
                        largest_amount="$line_amount"
                        largest_utxo="$utxo_id"
                        echo "[DEBUG] Batch $batch_num: New largest: $largest_utxo with amount $largest_amount"
                    fi
                fi
            done <<< "$all_utxos"
            
            if [ -n "$largest_utxo" ] && [ "$largest_amount" -gt 0 ]; then
                echo "[LOG] Found alternative UTXO: $largest_utxo with amount $largest_amount"
                current_batch_input_utxo="$largest_utxo"
                current_batch_input_utxo_amount="$largest_amount"
                utxo_queried_successfully=true
            else
                echo "[DEBUG] Batch $batch_num: No valid UTXO found in fallback parsing"
            fi
        fi
        
        if [ "$utxo_queried_successfully" = false ]; then
            echo "[DEBUG] CRITICAL ERROR: Batch $batch_num: No valid UTXO found. Aborting."
            exit 1
        fi
    fi
    
    echo "[LOG] Batch $batch_num: Input UTXO $current_batch_input_utxo has amount $current_batch_input_utxo_amount lovelace."

    # Now calculate fee using the actual UTXO (which may have reference scripts)
    echo "[LOG] Batch $batch_num: Building DUMMY transaction for fee calculation with actual UTXO. Outputs to fund: $actual_nodes_in_this_batch"
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
        batch_tx_fee=300000 # Increased fallback for reference scripts
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
            echo "[DEBUG] Batch $batch_num: ERROR calculating dynamic fee (Raw: '$calculated_batch_fee'). Using fallback 300000."
            batch_tx_fee=300000
        else
            # Base buffer for reference script overhead (fixed ~25k) + scaling buffer for larger batches
            reference_script_buffer=40000
            batch_scaling_buffer=$((actual_nodes_in_this_batch * 1000)) # 1k lovelace per output for safety
            total_buffer=$((reference_script_buffer + batch_scaling_buffer))
            
            batch_tx_fee=$((calculated_batch_fee + total_buffer))
            echo "[LOG] Batch $batch_num: Calculated Min Fee: $calculated_batch_fee, Reference Script Buffer: $reference_script_buffer, Batch Scaling Buffer: $batch_scaling_buffer, Total Fee: $batch_tx_fee"
        fi
        rm -f "$dummy_batch_tx_file"
    fi

    echo "[LOG] Batch $batch_num: Total output to registered nodes: $batch_total_output_lovelace, Calculated Fee: $batch_tx_fee"

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
        new_input_utxo_candidate=$(cardano-cli latest query utxo --address "$new_address" --testnet-magic 42 | /busybox grep lovelace | /busybox sort -k3 -nr | head -1 | /busybox awk '{print $1"#"$2}')
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

echo "[LOG] Generating Mainchain Cold Keys for Registered Nodes..."
for i in $(seq 1 $NUM_REGISTERED_NODES_TO_PROCESS); do
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

echo "[LOG] Querying and saving the first UTXO details for new address to /shared/genesis.utxo:"
# Query UTXOs and extract the first UTXO key from JSON format
cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}" --out-file /dev/stdout | /busybox grep -o '"[a-f0-9]\{64\}#[0-9]\+":' | head -1 | /busybox sed 's/"//g' | /busybox sed 's/://g' > /shared/genesis.utxo
cp /shared/genesis.utxo /runtime-values/genesis.utxo
echo "[LOG] Created /shared/genesis.utxo with value: $(cat /shared/genesis.utxo)"


# --- NEW: Register Nodes as Cardano SPOs and Delegate Stake ---
echo "[LOG] Starting Cardano SPO Registration and Delegation for all nodes..."

STAKE_ADDRESS_DEPOSIT_AMT=400000 # 2 ADA in lovelace for stake address registration. MODIFIED TO 0.4 ADA based on error analysis.
POOL_REG_DEPOSIT_AMT=500000000    # 500 ADA in lovelace for pool registration deposit

# Registered nodes (1-based index)
for i in $(seq 1 $NUM_REGISTERED_NODES_TO_PROCESS); do
    NODE_SPECIFIC_KEYS_DIR="/shared/node-keys/registered-${i}/keys"
    NODE_TYPE="registered"
    NODE_LOG_NAME="${NODE_TYPE}-${i}"
    NODE_PAYMENT_ADDRESS="${registered_node_payment_addresses[$((i-1))]}"
    NODE_STAKE_ADDRESS="${registered_node_stake_addresses[$((i-1))]}"
    NODE_COLD_VKEY="${NODE_SPECIFIC_KEYS_DIR}/cold.vkey"
    NODE_COLD_SKEY="${NODE_SPECIFIC_KEYS_DIR}/cold.skey"
    NODE_VRF_VKEY="${NODE_SPECIFIC_KEYS_DIR}/vrf.vkey"
    NODE_KES_VKEY="${NODE_SPECIFIC_KEYS_DIR}/kes.vkey"
    NODE_STAKE_VKEY="${NODE_SPECIFIC_KEYS_DIR}/stake.vkey"
    NODE_STAKE_SKEY="${NODE_SPECIFIC_KEYS_DIR}/stake.skey"
    NODE_PAYMENT_SKEY="${NODE_SPECIFIC_KEYS_DIR}/payment.skey"
    NODE_COLD_COUNTER="${NODE_SPECIFIC_KEYS_DIR}/cold.counter"

    echo "[LOG] Processing $NODE_LOG_NAME for SPO registration..."

    # Add this check
    if [ ! -f "$NODE_STAKE_VKEY" ]; then
        echo "[DEBUG] CRITICAL ERROR: Stake verification key file NOT FOUND for $NODE_LOG_NAME at path: $NODE_STAKE_VKEY. Cannot generate pool reg cert."
        if [ -f "/data/${NODE_LOG_NAME}_stake_reg.cert" ]; then rm -f "/data/${NODE_LOG_NAME}_stake_reg.cert"; fi
        continue
    else
        echo "[LOG] Stake verification key file FOUND for $NODE_LOG_NAME at path: $NODE_STAKE_VKEY."
    fi

    # 1. Query UTXO for transaction funding
    echo "[LOG] Querying UTXO for $NODE_LOG_NAME..."
    NODE_FUNDING_UTXO=""
    for attempt in {1..10}; do
        echo "[LOG] Querying address UTXOs for $NODE_LOG_NAME (Attempt $attempt)..."
        utxo_info=$(cardano-cli latest query utxo \
            --testnet-magic 42 --address "$NODE_PAYMENT_ADDRESS" --out-file /dev/stdout 2>&1)
        NODE_FUNDING_UTXO=$(echo "$utxo_info" | /busybox grep -o '[a-f0-9]\{64\}#[0-9]\+' | head -1)
        if [ -n "$NODE_FUNDING_UTXO" ]; then
            echo "[LOG] Found funding UTXO for $NODE_LOG_NAME: $NODE_FUNDING_UTXO"
            break
        else
            echo "[WARN] No UTXO found for $NODE_LOG_NAME at $NODE_PAYMENT_ADDRESS. Waiting 5s... (Attempt $attempt)"
            sleep 5
        fi
    done

    if [ -z "$NODE_FUNDING_UTXO" ]; then
        echo "[DEBUG] CRITICAL ERROR: Failed to find funding UTXO for $NODE_LOG_NAME. Cannot perform SPO registration/delegation. Skipping this node."
        continue # Skip to the next node if funding UTXO not found
    fi

    # Extract UTXO amount for fee calculation
    NODE_FUNDING_UTXO_AMOUNT=$(echo "$utxo_info" | /busybox grep "$NODE_FUNDING_UTXO" -A 20 | /busybox grep '"lovelace":' | /busybox grep -o '[0-9]\+' | head -1)
    echo "[LOG] $NODE_LOG_NAME Funding UTXO amount: $NODE_FUNDING_UTXO_AMOUNT lovelace."
    if ! [[ "$NODE_FUNDING_UTXO_AMOUNT" =~ ^[0-9]+$ ]] || [ "$NODE_FUNDING_UTXO_AMOUNT" -eq 0 ]; then
         echo "[DEBUG] CRITICAL ERROR: Failed to get valid UTXO amount for $NODE_LOG_NAME. Skipping this node."
         continue
    fi

    # 2. Generate Stake Address Registration Certificate
    echo "[LOG] Generating stake address registration certificate for $NODE_LOG_NAME..."
    STAKE_REG_CERT="/data/${NODE_LOG_NAME}_stake_reg.cert"
    if ! cardano-cli latest stake-address registration-certificate \
        --stake-verification-key-file "$NODE_STAKE_VKEY" \
        --key-reg-deposit-amt "$STAKE_ADDRESS_DEPOSIT_AMT" \
        --out-file "$STAKE_REG_CERT"; then
        echo "[DEBUG] ERROR: Failed to generate stake address registration certificate for $NODE_LOG_NAME. Skipping this node."
        continue
    fi

    # 3. Generate Stake Pool Registration Certificate
    echo "[LOG] Generating stake pool registration certificate for $NODE_LOG_NAME..."
    POOL_REG_CERT="/data/${NODE_LOG_NAME}_pool_reg.cert"
    POOL_ID=$(cardano-cli latest stake-pool id --cold-verification-key-file "$NODE_COLD_VKEY" --output-format hex)
    echo "[LOG] $NODE_LOG_NAME Pool ID: $POOL_ID"

    # Pool parameters (minimal for local env)
    PLEDGE=0 # No pledge required for this setup
    POOL_COST=0 # Minimal cost
    POOL_MARGIN="0/1000" # 0% margin

    echo "[DEBUG] Attempting to run stake-pool registration-certificate command..."

    if ! cardano-cli latest stake-pool registration-certificate \
        --cold-verification-key-file "$NODE_COLD_VKEY" \
        --vrf-verification-key-file "$NODE_VRF_VKEY" \
        --reward-account-verification-key-file "$NODE_STAKE_VKEY" \
        --pool-owner-stake-verification-key-file "$NODE_STAKE_VKEY" \
        --pool-pledge "$PLEDGE" \
        --pool-cost "$POOL_COST" \
        --pool-margin "$POOL_MARGIN" \
        --pool-relay-ipv4 127.0.0.1 \
        --pool-relay-port 30000 \
        --metadata-url "https://example.com/${NODE_LOG_NAME}.json" --metadata-hash 0000000000000000000000000000000000000000000000000000000000000000 \
        --testnet-magic 42 \
        --out-file "$POOL_REG_CERT"; then
        echo "[DEBUG] ERROR: Failed to generate stake pool registration certificate for $NODE_LOG_NAME. Skipping this node."
        rm -f "$STAKE_REG_CERT"
        continue
    fi

    # 4. Build Registration Transaction (Dummy for fee calculation)
    echo "[LOG] Building dummy registration transaction for $NODE_LOG_NAME fee calculation..."
    REG_TX_DUMMY="/data/${NODE_LOG_NAME}_reg_tx_dummy.raw"
    CHANGE_OUTPUT_DUMMY=1000000 # Placeholder for change (1 ADA)

    if ! cardano-cli latest transaction build-raw \
        --tx-in "$NODE_FUNDING_UTXO" \
        --tx-out "$NODE_PAYMENT_ADDRESS+$CHANGE_OUTPUT_DUMMY" \
        --certificate-file "$STAKE_REG_CERT" \
        --certificate-file "$POOL_REG_CERT" \
        --fee 0 \
        --out-file "$REG_TX_DUMMY"; then
        echo "[DEBUG] ERROR: Failed to build dummy registration transaction for $NODE_LOG_NAME. Skipping this node."
        rm -f "$STAKE_REG_CERT" "$POOL_REG_CERT"
        continue
    fi

    # 5. Calculate Fee
    echo "[LOG] Calculating registration transaction fee for $NODE_LOG_NAME..."
    NUM_REG_TX_INPUTS=1
    NUM_REG_TX_OUTPUTS=1 # Change output
    NUM_REG_TX_WITNESSES=3 # payment.skey, stake.skey, cold.skey

    CALCULATED_REG_FEE=$(cardano-cli latest transaction calculate-min-fee \
        --tx-body-file "$REG_TX_DUMMY" \
        --testnet-magic 42 \
        --protocol-params-file "$protocol_params_file" \
        --tx-in-count "$NUM_REG_TX_INPUTS" \
        --tx-out-count "$NUM_REG_TX_OUTPUTS" \
        --witness-count "$NUM_REG_TX_WITNESSES" | /busybox awk '{print $1}')

    rm -f "$REG_TX_DUMMY"

    if ! [[ "$CALCULATED_REG_FEE" =~ ^[0-9]+$ ]]; then
        echo "[DEBUG] ERROR: Failed to calculate registration transaction fee for $NODE_LOG_NAME. Using fallback fee."
        REG_FEE=500000 # Fallback fee
    else
        REG_FEE=$((CALCULATED_REG_FEE + 50000)) # Add buffer
        echo "[LOG] Calculated registration fee for $NODE_LOG_NAME: $REG_FEE"
    fi

    # 6. Calculate Change and Build Final Registration Transaction
    REG_TX_CHANGE=$((NODE_FUNDING_UTXO_AMOUNT - REG_FEE - STAKE_ADDRESS_DEPOSIT_AMT - POOL_REG_DEPOSIT_AMT))
    if [ "$REG_TX_CHANGE" -lt 1000000 ]; then # Ensure minimum change of 1 ADA
        echo "[DEBUG] CRITICAL ERROR: Registration transaction change for $NODE_LOG_NAME is too small ($REG_TX_CHANGE) after accounting for fee and deposits. Input: $NODE_FUNDING_UTXO_AMOUNT, Fee: $REG_FEE, StakeDeposit: $STAKE_ADDRESS_DEPOSIT_AMT, PoolDeposit: $POOL_REG_DEPOSIT_AMT. Skipping."
        rm -f "$STAKE_REG_CERT" "$POOL_REG_CERT"
        continue
    fi
    echo "[LOG] Registration transaction change for $NODE_LOG_NAME: $REG_TX_CHANGE"

    REG_TX_FINAL="/data/${NODE_LOG_NAME}_reg_tx.raw"
    if ! cardano-cli latest transaction build-raw \
        --tx-in "$NODE_FUNDING_UTXO" \
        --tx-out "$NODE_PAYMENT_ADDRESS+$REG_TX_CHANGE" \
        --certificate-file "$STAKE_REG_CERT" \
        --certificate-file "$POOL_REG_CERT" \
        --fee "$REG_FEE" \
        --out-file "$REG_TX_FINAL"; then
        echo "[DEBUG] ERROR: Failed to build final registration transaction for $NODE_LOG_NAME. Skipping this node."
        rm -f "$STAKE_REG_CERT" "$POOL_REG_CERT"
        continue
    fi

    # 7. Sign Registration Transaction
    echo "[LOG] Signing registration transaction for $NODE_LOG_NAME..."

    # Verify signing keys exist before attempting to sign
    echo "[DEBUG] Checking for signing keys for $NODE_LOG_NAME:"
    echo "[DEBUG]   Payment SKey: $NODE_PAYMENT_SKEY"
    echo "[DEBUG]   Stake SKey:   $NODE_STAKE_SKEY"
    echo "[DEBUG]   Cold SKey:    $NODE_COLD_SKEY"
    if [ ! -f "$NODE_PAYMENT_SKEY" ] || [ ! -f "$NODE_STAKE_SKEY" ] || [ ! -f "$NODE_COLD_SKEY" ]; then
        echo "[DEBUG] CRITICAL ERROR: One or more signing key files NOT FOUND for $NODE_LOG_NAME."
        echo "[DEBUG]   Payment SKey exists: $([ -f "$NODE_PAYMENT_SKEY" ] && echo true || echo false)"
        echo "[DEBUG]   Stake SKey exists:   $([ -f "$NODE_STAKE_SKEY" ] && echo true || echo false)"
        echo "[DEBUG]   Cold SKey exists:    $([ -f "$NODE_COLD_SKEY" ] && echo true || echo false)"
        # Clean up certs if they were created
        if [ -f "$STAKE_REG_CERT" ]; then rm -f "$STAKE_REG_CERT"; fi
        if [ -f "$POOL_REG_CERT" ]; then rm -f "$POOL_REG_CERT"; fi
        if [ -f "$REG_TX_FINAL" ]; then rm -f "$REG_TX_FINAL"; fi
        continue # Skip to next node
    fi

    REG_TX_SIGNED="/data/${NODE_LOG_NAME}_reg_tx.signed"
    if ! cardano-cli latest transaction sign \
        --tx-body-file "$REG_TX_FINAL" \
        --signing-key-file "$NODE_PAYMENT_SKEY" \
        --signing-key-file "$NODE_STAKE_SKEY" \
        --signing-key-file "$NODE_COLD_SKEY" \
        --testnet-magic 42 \
        --out-file "$REG_TX_SIGNED"; then
        echo "[DEBUG] ERROR: Failed to sign registration transaction for $NODE_LOG_NAME."
        echo "[DEBUG] Attempted to use keys:"
        echo "[DEBUG]   Payment SKey: $NODE_PAYMENT_SKEY"
        echo "[DEBUG]   Stake SKey:   $NODE_STAKE_SKEY"
        echo "[DEBUG]   Cold SKey:    $NODE_COLD_SKEY"
        rm -f "$STAKE_REG_CERT" "$POOL_REG_CERT" "$REG_TX_FINAL"
        continue
    fi

    # 8. Submit Registration Transaction
    echo "[LOG] Submitting registration transaction for $NODE_LOG_NAME..."
    SUBMITTED_REG=false
    for attempt in {1..5}; do
        if cardano-cli latest transaction submit --tx-file "$REG_TX_SIGNED" --testnet-magic 42; then
            echo "[LOG] Registration transaction submitted for $NODE_LOG_NAME."
            SUBMITTED_REG=true
            break
        else
            echo "[WARN] Attempt $attempt to submit registration transaction for $NODE_LOG_NAME failed. Retrying in 5s..."
            sleep 5
        fi
    done

    rm -f "$STAKE_REG_CERT" "$POOL_REG_CERT" "$REG_TX_FINAL" "$REG_TX_SIGNED" # Clean up

    if [ "$SUBMITTED_REG" = false ]; then
        echo "[DEBUG] CRITICAL ERROR: Failed to submit registration transaction for $NODE_LOG_NAME after multiple attempts. Skipping delegation for this node."
        continue # Skip delegation if registration failed
    fi

    # 9. Wait for confirmation and Query UTXO for Delegation Transaction
    echo "[LOG] Waiting 15 seconds for registration transaction for $NODE_LOG_NAME to confirm..."
    sleep 15

    echo "[LOG] Querying UTXO for delegation transaction for $NODE_LOG_NAME..."
    NODE_FUNDING_UTXO_DELEG=""
    for attempt in {1..10}; do
        echo "[LOG] Querying address UTXOs for $NODE_LOG_NAME (Delegation Attempt $attempt)..."
        utxo_info_deleg=$(cardano-cli latest query utxo \
            --testnet-magic 42 --address "$NODE_PAYMENT_ADDRESS" --out-file /dev/stdout 2>&1)
        NODE_FUNDING_UTXO_DELEG=$(echo "$utxo_info_deleg" | /busybox grep -o '[a-f0-9]\{64\}#[0-9]\+' | head -1)
        if [ -n "$NODE_FUNDING_UTXO_DELEG" ]; then
            echo "[LOG] Found funding UTXO for $NODE_LOG_NAME delegation: $NODE_FUNDING_UTXO_DELEG"
            break
        else
            echo "[WARN] No UTXO found for $NODE_LOG_NAME delegation at $NODE_PAYMENT_ADDRESS. Waiting 5s... (Attempt $attempt)"
            sleep 5
        fi
    done

     if [ -z "$NODE_FUNDING_UTXO_DELEG" ]; then
        echo "[DEBUG] CRITICAL ERROR: Failed to find funding UTXO for $NODE_LOG_NAME delegation. Cannot perform delegation. Skipping this node."
        continue # Skip delegation if funding UTXO not found
    fi
    NODE_FUNDING_UTXO_DELEG_AMOUNT=$(echo "$utxo_info_deleg" | /busybox grep "$NODE_FUNDING_UTXO_DELEG" -A 20 | /busybox grep '"lovelace":' | /busybox grep -o '[0-9]\+' | head -1)
    if ! [[ "$NODE_FUNDING_UTXO_DELEG_AMOUNT" =~ ^[0-9]+$ ]] || [ "$NODE_FUNDING_UTXO_DELEG_AMOUNT" -eq 0 ]; then
         echo "[DEBUG] CRITICAL ERROR: Failed to get valid UTXO amount for $NODE_LOG_NAME delegation. Skipping this node."
         continue
    fi
    echo "[LOG] $NODE_LOG_NAME Delegation Funding UTXO amount: $NODE_FUNDING_UTXO_DELEG_AMOUNT lovelace."


    # 10. Generate Delegation Certificate
    echo "[LOG] Generating delegation certificate for $NODE_LOG_NAME to pool $POOL_ID..."
    DELEG_CERT="/data/${NODE_LOG_NAME}_deleg.cert"
     if ! cardano-cli latest stake-address stake-delegation-certificate \
        --stake-verification-key-file "$NODE_STAKE_VKEY" \
        --stake-pool-id "$POOL_ID" \
        --out-file "$DELEG_CERT"; then
        echo "[DEBUG] ERROR: Failed to generate delegation certificate for $NODE_LOG_NAME. Skipping this node."
        continue
    fi

    # 11. Build Delegation Transaction (Dummy for fee calculation)
    echo "[LOG] Building dummy delegation transaction for $NODE_LOG_NAME fee calculation..."
    DELEG_TX_DUMMY="/data/${NODE_LOG_NAME}_deleg_tx_dummy.raw"
    CHANGE_OUTPUT_DELEG_DUMMY=1000000 # Placeholder for change (1 ADA)

    if ! cardano-cli latest transaction build-raw \
        --tx-in "$NODE_FUNDING_UTXO_DELEG" \
        --tx-out "$NODE_PAYMENT_ADDRESS+$CHANGE_OUTPUT_DELEG_DUMMY" \
        --certificate-file "$DELEG_CERT" \
        --fee 0 \
        --out-file "$DELEG_TX_DUMMY"; then
        echo "[DEBUG] ERROR: Failed to build dummy delegation transaction for $NODE_LOG_NAME. Skipping this node."
        rm -f "$DELEG_CERT"
        continue
    fi

    # 12. Calculate Fee
    echo "[LOG] Calculating delegation transaction fee for $NODE_LOG_NAME..."
    NUM_DELEG_TX_INPUTS=1
    NUM_DELEG_TX_OUTPUTS=1 # Change output
    NUM_DELEG_TX_WITNESSES=2 # payment.skey, stake.skey

    CALCULATED_DELEG_FEE=$(cardano-cli latest transaction calculate-min-fee \
        --tx-body-file "$DELEG_TX_DUMMY" \
        --testnet-magic 42 \
        --protocol-params-file "$protocol_params_file" \
        --tx-in-count "$NUM_DELEG_TX_INPUTS" \
        --tx-out-count "$NUM_DELEG_TX_OUTPUTS" \
        --witness-count "$NUM_DELEG_TX_WITNESSES" | /busybox awk '{print $1}')

    rm -f "$DELEG_TX_DUMMY"

    if ! [[ "$CALCULATED_DELEG_FEE" =~ ^[0-9]+$ ]]; then
        echo "[DEBUG] ERROR: Failed to calculate delegation transaction fee for $NODE_LOG_NAME. Using fallback fee."
        DELEG_FEE=300000 # Fallback fee
    else
        DELEG_FEE=$((CALCULATED_DELEG_FEE + 50000)) # Add buffer
        echo "[LOG] Calculated delegation fee for $NODE_LOG_NAME: $DELEG_FEE"
    fi

    # 13. Calculate Change and Build Final Delegation Transaction
    DELEG_TX_CHANGE=$((NODE_FUNDING_UTXO_DELEG_AMOUNT - DELEG_FEE))
     if [ "$DELEG_TX_CHANGE" -lt 1000000 ]; then # Ensure minimum change
        echo "[DEBUG] CRITICAL ERROR: Delegation transaction change for $NODE_LOG_NAME is too small ($DELEG_TX_CHANGE). Skipping."
        rm -f "$DELEG_CERT"
        continue
    fi
    echo "[LOG] Delegation transaction change for $NODE_LOG_NAME: $DELEG_TX_CHANGE"


    DELEG_TX_FINAL="/data/${NODE_LOG_NAME}_deleg_tx.raw"
    if ! cardano-cli latest transaction build-raw \
        --tx-in "$NODE_FUNDING_UTXO_DELEG" \
        --tx-out "$NODE_PAYMENT_ADDRESS+$DELEG_TX_CHANGE" \
        --certificate-file "$DELEG_CERT" \
        --fee "$DELEG_FEE" \
        --out-file "$DELEG_TX_FINAL"; then
        echo "[DEBUG] ERROR: Failed to build final delegation transaction for $NODE_LOG_NAME. Skipping this node."
        rm -f "$DELEG_CERT"
        continue
    fi

    # 14. Sign Delegation Transaction
    echo "[LOG] Signing delegation transaction for $NODE_LOG_NAME..."

    # Verify signing keys exist before attempting to sign
    echo "[DEBUG] Checking for signing keys for $NODE_LOG_NAME (delegation):"
    echo "[DEBUG]   Payment SKey: $NODE_PAYMENT_SKEY"
    echo "[DEBUG]   Stake SKey:   $NODE_STAKE_SKEY"
    echo "[DEBUG]   Cold SKey:    $NODE_COLD_SKEY"
    if [ ! -f "$NODE_PAYMENT_SKEY" ] || [ ! -f "$NODE_STAKE_SKEY" ]; then
        echo "[DEBUG] CRITICAL ERROR: One or more signing key files NOT FOUND for $NODE_LOG_NAME (delegation)."
        echo "[DEBUG]   Payment SKey exists: $([ -f "$NODE_PAYMENT_SKEY" ] && echo true || echo false)"
        echo "[DEBUG]   Stake SKey exists:   $([ -f "$NODE_STAKE_SKEY" ] && echo true || echo false)"
        rm -f "$DELEG_CERT" "$DELEG_TX_FINAL"
        continue
    fi

    DELEG_TX_SIGNED="/data/${NODE_LOG_NAME}_deleg_tx.signed"

     if ! cardano-cli latest transaction sign \
        --tx-body-file "$DELEG_TX_FINAL" \
        --signing-key-file "$NODE_PAYMENT_SKEY" \
        --signing-key-file "$NODE_STAKE_SKEY" \
        --testnet-magic 42 \
        --out-file "$DELEG_TX_SIGNED"; then
        echo "[DEBUG] ERROR: Failed to sign delegation transaction for $NODE_LOG_NAME."
        echo "[DEBUG] Attempted to use keys:"
        echo "[DEBUG]   Payment SKey: $NODE_PAYMENT_SKEY"
        echo "[DEBUG]   Stake SKey:   $NODE_STAKE_SKEY"
        rm -f "$DELEG_CERT" "$DELEG_TX_FINAL"
        continue
    fi

    DELEG_TX_ID=$(cardano-cli latest transaction txid --tx-file "$DELEG_TX_SIGNED")
    if [ -z "$DELEG_TX_ID" ]; then
        echo "[DEBUG] CRITICAL ERROR: Could not get TxID for delegation tx for $NODE_LOG_NAME. Cannot save final UTXO."
        rm -f "$DELEG_CERT" "$DELEG_TX_FINAL" "$DELEG_TX_SIGNED"
        continue
    fi

    # 15. Submit Delegation Transaction
    echo "[LOG] Submitting delegation transaction for $NODE_LOG_NAME..."
    SUBMITTED_DELEG=false
    for attempt in {1..5}; do
        if cardano-cli latest transaction submit --tx-file "$DELEG_TX_SIGNED" --testnet-magic 42; then
            echo "[LOG] Delegation transaction submitted for $NODE_LOG_NAME."
            SUBMITTED_DELEG=true
            break
        else
            echo "[WARN] Attempt $attempt to submit delegation transaction for $NODE_LOG_NAME failed. Retrying in 5s..."
            sleep 5
        fi
    done

    rm -f "$DELEG_CERT" "$DELEG_TX_FINAL" "$DELEG_TX_SIGNED" # Clean up

    if [ "$SUBMITTED_DELEG" = false ]; then
        echo "[DEBUG] CRITICAL ERROR: Failed to submit delegation transaction for $NODE_LOG_NAME after multiple attempts."
    else
        FINAL_UTXO="${DELEG_TX_ID}#0"
        echo "[LOG] Saving final UTXO for $NODE_LOG_NAME to /shared/registered-${i}.utxo: $FINAL_UTXO"
        echo "$FINAL_UTXO" > "/shared/registered-${i}.utxo"

        echo "[LOG] Waiting for delegation transaction for $NODE_LOG_NAME ($DELEG_TX_ID) to be confirmed on-chain..."
        for attempt in {1..20}; do
            # Check if the UTXO created by the delegation transaction exists.
            if cardano-cli latest query utxo --tx-in "$DELEG_TX_ID#0" --testnet-magic 42 | /busybox grep -q "$DELEG_TX_ID"; then
                echo "[LOG] Delegation transaction for $NODE_LOG_NAME confirmed."
                break
            fi
            if [ "$attempt" -eq 20 ]; then
                echo "[DEBUG] CRITICAL ERROR: Delegation transaction for $NODE_LOG_NAME was not confirmed on-chain after 100 seconds."
            fi
            sleep 5
        done

        echo "[LOG] Waiting for stake delegation for $NODE_LOG_NAME to become active (2 epochs)..."
        sleep 250

        echo "[LOG] Querying stake address info for $NODE_LOG_NAME to verify delegation..."
        cardano-cli latest query stake-address-info \
            --address "$NODE_STAKE_ADDRESS" \
            --testnet-magic 42 --out-file /dev/stdout || echo "Query stake-address-info failed for $NODE_LOG_NAME"
    fi

    echo "[LOG] Completed SPO registration and delegation process for $NODE_LOG_NAME."
    echo "---" # Separator for logs

done # End loop for registered nodes

echo "[LOG] Completed SPO Registration and Delegation for all nodes."
# --- END NEW: Register Nodes as Cardano SPOs and Delegate Stake ---

echo "[LOG] Creating /shared/cardano.ready signal file."
touch /shared/cardano.ready

echo "[LOG] Cardano node entrypoint script finished. Waiting for node process to terminate (if it does)."
wait
