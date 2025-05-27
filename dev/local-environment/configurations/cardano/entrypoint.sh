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
echo "[LOG] Finalizing UTXO files for registered nodes after all batch funding..."
for i in {1..300}; do
    node_unique_address="${registered_node_payment_addresses[$((i-1))]}"
    # It's critical that the UTXO exists now. Add some retries just in case of chain lag.
    final_utxo_found=false
    for attempt in {1..5}; do # Try up to 5 times
        echo "[LOG] Querying final UTXO for registered-$i at $node_unique_address (Attempt $attempt)..."
        # Query all UTXOs at the address, take the first one (usually only one after funding)
        # Output format of cardano-cli query utxo:
        #                            TxHash                                 TxIx        Amount
        # ----------------------------------------------------------------------------------------
        # d8d93399a255e0d69781a25a6e0a0f46548c43357c15d0aa9e0a89c219495659     0        10000000 lovelace + ...
        
        # DEBUG: Capture raw output
        raw_cli_output_file="/tmp/raw_cli_output_registered_${i}_attempt_${attempt}.txt"
        echo "[DEBUG] Attempting to run for $node_unique_address (Attempt $attempt): cardano-cli latest query utxo --testnet-magic 42 --address \\"$node_unique_address\\" --out-file /dev/stdout"
        cardano-cli latest query utxo --testnet-magic 42 --address "$node_unique_address" --out-file /dev/stdout > "$raw_cli_output_file" 2>&1

        echo "[DEBUG] Raw output from cardano-cli for $node_unique_address (Attempt $attempt) captured in $raw_cli_output_file:"
        cat "$raw_cli_output_file"

        node_utxo_final=$(cat "$raw_cli_output_file" | /busybox awk 'NR>2 {print $1 "#" $2; exit}')
        echo "[DEBUG] Parsed node_utxo_final by awk: [$node_utxo_final]"
        
        if [ -n "$node_utxo_final" ]; then
            # Basic validation that it looks like a TxHash#TxIx
            if [[ "$node_utxo_final" =~ ^[a-f0-9]{64}#[0-9]+$ ]]; then
                echo "$node_utxo_final" > "/shared/registered-${i}.utxo"
                echo "[LOG] Successfully updated /shared/registered-${i}.utxo with: $node_utxo_final"
                final_utxo_found=true
                break
            else
                echo "[WARN] Attempt $attempt: For registered-$i at $node_unique_address, query output [$node_utxo_final] does not look like TxHash#TxIx. Retrying..."
                node_utxo_final="" # Clear it so the -n check fails if next attempt also bad
            fi
        else
            echo "[WARN] Attempt $attempt: No UTXO found yet for registered-$i at $node_unique_address. Sleeping 5s..."
            sleep 5
        fi
    done

    if [ "$final_utxo_found" = false ]; then
        echo "[ERROR] CRITICAL: Failed to find UTXO for registered-$i at $node_unique_address after multiple attempts. /shared/registered-${i}.utxo will be empty. This will likely cause registration to fail for this node."
        # Ensure the file is empty if no UTXO found, to prevent stale data issues
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
        # Optionally exit here if this is critical
    fi
done
echo "[LOG] Finished generating mainchain cold keys."

echo "[LOG] Creating /shared/cardano.ready signal file."
touch /shared/cardano.ready

echo "[LOG] Cardano node entrypoint script finished. Waiting for node process to terminate (if it does)."
wait
