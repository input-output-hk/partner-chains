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
    echo "[DEBUG] Value of variable '$var_name' (which is \\$$var_name) before dereference: $(eval echo \"\\$$var_name\")"
    amount_permissioned="${!var_name}"
    echo "[DEBUG] amount_permissioned for iteration $i after dereference is: '$amount_permissioned'"
    total_output=$((total_output + amount_permissioned))
done
echo "[LOG] total_output after permissioned nodes = $total_output"

echo "[LOG] Adding tx_out5_lovelace and tx_out6 to total_output."
total_output=$((total_output + tx_out5_lovelace + tx_out6))
echo "[LOG] Final total_output before fee = $total_output"

echo "[LOG] Calculating total output for the main funding transaction." # This log might be slightly misplaced now but ok
echo "[LOG] Main transaction: total_output=$total_output, fee=$fee, change=$change" # Fee and change not yet calculated here

fee=1000000
echo "[LOG] Fee set to: $fee"

# Calculate remaining balance to return to the genesis address
change=$((tx_in_amount - total_output - fee))
echo "[LOG] Change calculated: $change (tx_in_amount=$tx_in_amount - total_output=$total_output - fee=$fee)"

# Assemble all --tx-out parameters
tx_out_params_array=()
tx_out_params_array+=(--tx-out "$new_address+$tx_out1")
tx_out_params_array+=(--tx-out "$new_address+$tx_out2")
tx_out_params_array+=(--tx-out "$new_address+$tx_out3")
tx_out_params_array+=(--tx-out "$new_address+$tx_out4")

# Permissioned nodes outputs (still to $new_address)
for i in {1..10}; do
    var_name="tx_out${i}_permissioned"
    amount_permissioned="${!var_name}"
    tx_out_params_array+=(--tx-out "$new_address+$amount_permissioned")
done

# Output with native token for new_address
tx_out_params_array+=(--tx-out "$new_address+$tx_out5_lovelace+$tx_out5_reward_token")

# Output for vfunction_address (this one sends to a different address)
tx_out_params_array+=(--tx-out "$vfunction_address+$tx_out6")

# Change output for new_address
tx_out_params_array+=(--tx-out "$new_address+$change")

# Build the raw transaction
cardano-cli latest transaction build-raw \
  --tx-in $tx_in1 \
  "${tx_out_params_array[@]}" \
  --tx-out-reference-script-file /shared/v-function.script \
  --minting-script-file /shared/reward_token_policy.script \
  --mint "$tx_out5_reward_token" \
  --fee $fee \
  --out-file /data/tx.raw

echo "[LOG] Building the main funding transaction raw file..."
echo "[LOG] Main funding transaction raw file created at /data/tx.raw."

# Sign the transaction
cardano-cli latest transaction sign \
  --tx-body-file /data/tx.raw \
  --signing-key-file /shared/shelley/genesis-utxo.skey \
  --signing-key-file /keys/funded_address.skey \
  --testnet-magic 42 \
  --out-file /data/tx.signed

echo "[LOG] Signing the main funding transaction..."
echo "[LOG] Main funding transaction signed at /data/tx.signed."

echo "[LOG] Displaying signed main transaction details:"
cat /data/tx.signed

echo "[LOG] Submitting the main funding transaction..."
cardano-cli latest transaction submit \
  --tx-file /data/tx.signed \
  --testnet-magic 42

echo "[LOG] Main funding transaction submitted."

echo "[LOG] Waiting 20 seconds for the main transaction to process..."
sleep 20
echo "Balance:"

# Query UTXOs at new_address
echo "[LOG] Querying UTXO for new_address:"
cardano-cli latest query utxo \
  --testnet-magic 42 \
  --address $new_address

echo "[LOG] Saving FUNDED_ADDRESS to /shared/FUNDED_ADDRESS: $new_address"
echo $new_address > /shared/FUNDED_ADDRESS
echo "Created /shared/FUNDED_ADDRESS with value: $new_address"

# Registered nodes UTXOs - now query each unique address
echo "[LOG] Saving UTXO details for registered nodes (these will be empty until batch funding completes for each node)..."
for i in {1..300}; do
    node_unique_address="${registered_node_payment_addresses[$((i-1))]}"
    echo "[LOG] Querying UTXO for registered-$i at address $node_unique_address..."
    # Query the specific address for this node. Expecting one UTXO from the funding.
    # Use awk to get the first UTXO (txhash#txid) after the header lines.
    # Redirect stderr to /dev/null to suppress "No UTXOs found" if the address isn't funded yet, which would be an error.
    node_utxo=$(cardano-cli latest query utxo --testnet-magic 42 --address "$node_unique_address" 2>/dev/null | /busybox awk 'NR==3 {print $1 "#" $2}') 

    if [ -z "$node_utxo" ]; then
        echo "Error: No UTXO found for registered-$i at address $node_unique_address. Funding might have failed or transaction not processed."
        # For now, we'll create an empty file to avoid breaking partner-chains-setup, but it will fail there.
        echo "" > "/shared/registered-${i}.utxo"
    else
        echo "$node_utxo" > "/shared/registered-${i}.utxo"
        echo "Saved UTXO for registered-$i: $node_utxo to /shared/registered-${i}.utxo"
    fi
done
echo "[LOG] Finished attempting to save UTXOs for registered nodes."

echo "[LOG] Querying and saving the first UTXO from $new_address to /shared/genesis.utxo..."
cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}" | /busybox awk 'NR>2 { print $1 "#" $2; exit }' > /shared/genesis.utxo
# Check if the file was created and is not empty
if [ -s "/shared/genesis.utxo" ]; then
    echo "[LOG] Successfully created /shared/genesis.utxo: $(cat /shared/genesis.utxo)"
    cp /shared/genesis.utxo /runtime-values/genesis.utxo
else
    echo "[LOG] ERROR: Failed to create or find UTXO for /shared/genesis.utxo from $new_address"
    echo "[LOG] Full UTXO query output for $new_address:"
    cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}"
fi

echo "[LOG] Starting batch funding for 300 registered nodes... Batch size: $batch_size, Num batches: $num_batches"
batch_size=10
num_batches=$(( (300 + batch_size - 1) / batch_size )) # Calculate number of batches (ceiling division)
current_funding_utxo="" # Will be populated from $new_address

# Get an initial UTXO from $new_address to start funding batches
# This expects /shared/genesis.utxo to have been created from a UTXO at $new_address
if [ -s "/shared/genesis.utxo" ]; then
    current_funding_utxo=$(cat /shared/genesis.utxo)
    echo "[LOG] Initial funding UTXO for batches: $current_funding_utxo"
else
    echo "[LOG] CRITICAL ERROR: No initial funding UTXO available (genesis.utxo empty and no alternative found). Cannot start batch funding."
    # Consider exiting if this is fatal: exit 1
fi

# Amount to send to each registered node in the batch
amount_per_registered_node=1000000000 # Same as defined earlier: tx_out${i}_registered

for batch_num in $(seq 1 $num_batches); do
    echo "[LOG] Processing Batch $batch_num of $num_batches..."
    start_index=$(( (batch_num - 1) * batch_size ))
    end_index=$(( start_index + batch_size - 1 ))
    if [ $end_index -ge 300 ]; then
        end_index=299 # Max index is 299 for 300 nodes (0-299)
    fi

    echo "[LOG] Batch $batch_num: Nodes from index $start_index to $end_index."

    if [ -z "$current_funding_utxo" ]; then
        echo "[LOG] Batch $batch_num: No funding UTXO available. Attempting to re-query..."
        # Attempt to re-query for a UTXO at $new_address
        utxo_query_output=$(cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}" | /busybox awk 'NR>2 {print $1 "#" $2; exit}')
        if [ -n "$utxo_query_output" ]; then
            current_funding_utxo=$utxo_query_output
            echo "[LOG] Batch $batch_num: Found new funding UTXO: $current_funding_utxo"
        else
            echo "[LOG] Batch $batch_num: CRITICAL ERROR - Still no funding UTXO. Aborting further batches."
            break # Exit the batch loop
        fi
    fi
    
    current_tx_in="$current_funding_utxo"
    echo "[LOG] Batch $batch_num: Using input UTXO: $current_tx_in"
    
    # Query the input UTXO to get its amount
    tx_in_detail=$(cardano-cli latest query utxo --testnet-magic 42 --tx-in "$current_tx_in" --out-file /dev/stdout | /busybox grep lovelace | /busybox awk '{print $NF}')
    if ! [[ "$tx_in_detail" =~ ^[0-9]+$ ]]; then # Check if it's a number
        echo "Error: Could not determine amount for input UTXO $current_tx_in. Trying to query all UTXOs at $new_address instead."
        # Fallback: try to get any UTXO's amount if specific one fails
        tx_in_detail=$(cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}" | /busybox grep lovelace | /busybox head -n1 | /busybox awk '{print $NF}')
        if ! [[ "$tx_in_detail" =~ ^[0-9]+$ ]]; then
             echo "CRITICAL ERROR: Failed to determine input amount for batch $batch_num from $new_address. Aborting."
             break
        fi
         echo "[LOG] Batch $batch_num: Determined input UTXO amount (fallback): $tx_in_detail"
    else
        echo "[LOG] Batch $batch_num: Input UTXO amount: $tx_in_detail"
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

    echo "[LOG] Batch $batch_num: Processing $nodes_in_this_batch nodes. Total output for nodes: $batch_total_output"

    if [ $nodes_in_this_batch -eq 0 ]; then
        echo "[LOG] Batch $batch_num: No nodes in this batch. Skipping."
        continue
    fi

    batch_fee=200000 # Estimate fee per batch, can be refined
    batch_change=$((current_tx_in_amount - batch_total_output - batch_fee))

    echo "[LOG] Batch $batch_num: Fee=$batch_fee, Change=$batch_change"

    if [ $batch_change -lt 0 ]; then
        echo "[LOG] Batch $batch_num: ERROR - Not enough funds. Skipping batch. Required: $((batch_total_output + batch_fee)), Available: $current_tx_in_amount"
        current_funding_utxo="" 
        continue
    fi

    batch_tx_out_params+=(--tx-out "$new_address+$batch_change")

    echo "[LOG] Batch $batch_num: Building transaction..."
    cardano-cli latest transaction build-raw \
      --tx-in "$current_tx_in" \
      "${batch_tx_out_params[@]}" \
      --fee "$batch_fee" \
      --out-file "/data/tx_batch_${batch_num}.raw"

    echo "[LOG] Batch $batch_num: Signing transaction..."
    cardano-cli latest transaction sign \
      --tx-body-file "/data/tx_batch_${batch_num}.raw" \
      --signing-key-file /keys/funded_address.skey \
      --testnet-magic 42 \
      --out-file "/data/tx_batch_${batch_num}.signed"

    echo "[LOG] Batch $batch_num: Submitting transaction..."
    cardano-cli latest transaction submit \
      --tx-file "/data/tx_batch_${batch_num}.signed" \
      --testnet-magic 42
    
    echo "[LOG] Batch $batch_num: Waiting 15 seconds for processing..."
    sleep 15

    echo "[LOG] Batch $batch_num: Attempting to find new change UTXO for next batch..."
    # Find the new change UTXO at $new_address to use for the next batch
    # This is a simple way; a more robust way would be to parse the txid and find the exact change UTXO
    new_utxo_query=$(cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}" | /busybox awk -v old_tx_hash="${current_tx_in%#*}" 'NR>2 && $1 != old_tx_hash {print $1 "#" $2; exit}')
    if [ -n "$new_utxo_query" ]; then
        current_funding_utxo="$new_utxo_query"
        echo "[LOG] Batch $batch_num: New funding UTXO for next batch: $current_funding_utxo (from specific change)"
    else
        # If specific change UTXO not found, try to get any UTXO from $new_address
        current_funding_utxo=$(cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}" | /busybox awk 'NR>2 {print $1 "#" $2; exit}')
        if [ -z "$current_funding_utxo" ]; then
            echo "[LOG] Batch $batch_num: CRITICAL ERROR - Could not find a new UTXO at $new_address after batch $batch_num. Aborting further batches."
            break
        else
            echo "[LOG] Batch $batch_num: Found a fallback UTXO at $new_address for next batch: $current_funding_utxo (from fallback query)"
        fi
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
