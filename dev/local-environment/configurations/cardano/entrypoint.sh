#! /bin/bash

chmod 600 /keys/*
chmod +x /busybox
chmod 777 /shared

echo "Calculating target time for synchronised chain start..."

# Local env Partner Chains epochs are 30 seconds long. PC and MC epochs have to align. The following line makes MC epoch 0 start at some PC epoch start.
target_time=$(( ($(date +%s) / 30 + 1) * 30 ))
echo "$target_time" > /shared/cardano.start
byron_startTime=$target_time
shelley_systemStart=$(date --utc +"%Y-%m-%dT%H:%M:%SZ" --date="@$target_time")

/busybox sed "s/\"startTime\": [0-9]*/\"startTime\": $byron_startTime/" /shared/byron/genesis.json.base > /shared/byron/genesis.json
echo "Updated startTime value in Byron genesis.json to: $byron_startTime"

/busybox sed "s/\"systemStart\": \"[^\"]*\"/\"systemStart\": \"$shelley_systemStart\"/" /shared/shelley/genesis.json.base > /shared/shelley/genesis.json
echo "Updated systemStart value in Shelley genesis.json to: $shelley_systemStart"

echo "Parsing epochLength and slotLength from Shelley genesis.json..."
/busybox awk -F':|,' '/"epochLength"/ {print $2}' /shared/shelley/genesis.json.base > /shared/mc-epoch-length
echo "Created /shared/mc-epoch-length with value: $(cat /shared/mc-epoch-length)"

/busybox awk -F':|,' '/"slotLength"/ {print $2}' /shared/shelley/genesis.json.base > /shared/mc-slot-length
echo "Created /shared/mc-slot-length with value: $(cat /shared/mc-slot-length)"

cp /shared/conway/genesis.conway.json.base /shared/conway/genesis.conway.json
cp /shared/shelley/genesis.alonzo.json.base /shared/shelley/genesis.alonzo.json
echo "Created /shared/conway/genesis.conway.json and /shared/shelley/genesis.alonzo.json"

byron_hash=$(/bin/cardano-cli byron genesis print-genesis-hash --genesis-json /shared/byron/genesis.json)
shelley_hash=$(/bin/cardano-cli latest genesis hash --genesis /shared/shelley/genesis.json)
alonzo_hash=$(/bin/cardano-cli latest genesis hash --genesis /shared/shelley/genesis.alonzo.json)
conway_hash=$(/bin/cardano-cli latest genesis hash --genesis /shared/conway/genesis.conway.json)

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

byron_startTimeMillis=$(($byron_startTime * 1000))
echo $byron_startTimeMillis > /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS
echo $byron_startTimeMillis > /runtime-values/MC__FIRST_EPOCH_TIMESTAMP_MILLIS
echo "Created /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS with value: $byron_startTimeMillis"

echo "Current time is now: $(date +"%H:%M:%S.%3N"). Starting node..."

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

echo "Waiting for node.socket..."

while true; do
    if [ -e "/data/node.socket" ]; then
        break
    else
        sleep 1
    fi
done

echo "Preparing native token owned by 'funded_address.skey'"
# Policy requires that mints are signed by the funded_address.skey (key hash is e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b)
reward_token_policy_id=$(cardano-cli latest transaction policyid --script-file ./shared/reward_token_policy.script)
# hex of "Reward token"
reward_token_asset_name="52657761726420746f6b656e"
echo "Generating new address and funding it with 2x1000 Ada and 10 Ada + 1000000 reward token ($reward_token_policy_id.$reward_token_asset_name)"

new_address=$(cardano-cli latest address build \
  --payment-verification-key-file /keys/funded_address.vkey \
  --testnet-magic 42)

echo "New address created: $new_address"

# Array to store payment addresses for registered nodes
registered_node_payment_addresses=()

echo "Generating Payment Keypairs and Addresses for 300 Registered Nodes..."
for i in {1..300}; do
    NODE_SPECIFIC_KEYS_DIR="/shared/node-keys/registered-${i}/keys"
    mkdir -p "$NODE_SPECIFIC_KEYS_DIR" # Ensure cold key dir also exists if not created yet

    echo "Generating payment keys for registered-$i in $NODE_SPECIFIC_KEYS_DIR..."
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
echo "Completed generation of payment keypairs and addresses for registered nodes."

# An address that will keep an UTXO with script of a test V-function, related to the SPO rewards. See v-function.script file.
vfunction_address="addr_test1vzuasm5nqzh7n909f7wang7apjprpg29l2f9sk6shlt84rqep6nyc"

# Define the UTXO details and amounts
tx_in1="781cb948a37c7c38b43872af9b1e22135a94826eafd3740260a6db0a303885d8#0"
tx_in_amount=29993040000000000

# Define output amounts
tx_out1=1000000000 # partner-chains-setup
tx_out2=1000000000 # partner-chains-setup (backup)
tx_out3=1000000000 # partner-chains-setup (additional)
tx_out4=1000000000 # partner-chains-setup (extra)

# Fund 10 permissioned nodes
for i in {1..10}; do
    eval "tx_out${i}_permissioned=1000000000 # permissioned-$i"
done

# Fund 300 registered nodes
for i in {1..300}; do
    eval "tx_out${i}_registered=1000000000 # registered-$i"
done

tx_out5_lovelace=10000000
tx_out5_reward_token="1000000 $reward_token_policy_id.$reward_token_asset_name"
tx_out6=10000000

# Calculate total output
total_output=$((tx_out1 + tx_out2 + tx_out3 + tx_out4))
for i in {1..10}; do
    eval "amount_permissioned_node=\\$tx_out${i}_permissioned"
    total_output=$((total_output + amount_permissioned_node))
done
for i in {1..300}; do
    eval "amount_registered_node=\\$tx_out${i}_registered"
    total_output=$((total_output + amount_registered_node))
done
total_output=$((total_output + tx_out5_lovelace + tx_out6))

fee=1000000

# Calculate remaining balance to return to the genesis address
change=$((tx_in_amount - total_output - fee))

# Assemble all --tx-out parameters
tx_out_params_array=()
tx_out_params_array+=(--tx-out "$new_address+$tx_out1")
tx_out_params_array+=(--tx-out "$new_address+$tx_out2")
tx_out_params_array+=(--tx-out "$new_address+$tx_out3")
tx_out_params_array+=(--tx-out "$new_address+$tx_out4")

# Permissioned nodes outputs (still to $new_address, assuming they are funded differently or their tx fees are paid by a central wallet)
for i in {1..10}; do
    eval "amount_permissioned=\\$tx_out${i}_permissioned"
    tx_out_params_array+=(--tx-out "$new_address+$amount_permissioned")
done

# Registered nodes outputs - now to their unique payment addresses
for i in {1..300}; do
    eval "amount_registered=\\$tx_out${i}_registered"
    # Use the unique address for this node. Array is 0-indexed.
    node_unique_address="${registered_node_payment_addresses[$((i-1))]}"
    tx_out_params_array+=(--tx-out "$node_unique_address+$amount_registered")
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

# Sign the transaction
cardano-cli latest transaction sign \
  --tx-body-file /data/tx.raw \
  --signing-key-file /shared/shelley/genesis-utxo.skey \
  --signing-key-file /keys/funded_address.skey \
  --testnet-magic 42 \
  --out-file /data/tx.signed

cat /data/tx.signed

echo "Submitting transaction..."
cardano-cli latest transaction submit \
  --tx-file /data/tx.signed \
  --testnet-magic 42

echo "Transaction submitted to fund governance authority and all validator nodes. Waiting 20 seconds for transaction to process..."
sleep 20
echo "Balance:"

# Query UTXOs at new_address
echo "Querying UTXO for new_address:"
cardano-cli latest query utxo \
  --testnet-magic 42 \
  --address $new_address

# Save dynamic values to shared config volume for other nodes to use
echo $new_address > /shared/FUNDED_ADDRESS
echo "Created /shared/FUNDED_ADDRESS with value: $new_address"

utxo_list=($(cardano-cli latest query utxo --testnet-magic 42 --address "$new_address" | /busybox awk 'NR>2 { print $1 "#" $2 }'))

# Expected UTXOs at $new_address: 4 general + 10 permissioned + 1 token output + 1 change output = 16
# The 300 registered node UTXOs are now at their own addresses.
num_expected_utxos_at_new_address=16 
if [ ${#utxo_list[@]} -lt $num_expected_utxos_at_new_address ]; then
    echo "Warning: Expected at least $num_expected_utxos_at_new_address UTXOs at $new_address (excluding registered nodes), but found ${#utxo_list[@]}. UTXO files might be incorrect for permissioned nodes or genesis."
    echo "All UTXOs found at $new_address:"
    printf '%s\n' "${utxo_list[@]}"
fi

# Registered nodes UTXOs - now query each unique address
echo "Saving UTXO details for registered nodes (each at their unique address)..."
for i in {1..300}; do
    node_unique_address="${registered_node_payment_addresses[$((i-1))]}"
    echo "Querying UTXO for registered-$i at address $node_unique_address..."
    # Query the specific address for this node. Expecting one UTXO from the funding.
    # Use awk to get the first UTXO (txhash#txid) after the header lines.
    # Redirect stderr to /dev/null to suppress "No UTXOs found" if the address isn't funded yet, which would be an error.
    node_utxo=$(cardano-cli latest query utxo --testnet-magic 42 --address "$node_unique_address" 2>/dev/null | /busybox awk 'NR==3 {print $1 "#" $2}') 

    if [ -z "$node_utxo" ]; then
        echo "Error: No UTXO found for registered-$i at address $node_unique_address. Funding might have failed or transaction not processed."
        # Consider exiting or adding a retry/wait loop here if this is expected to happen transiently
        # For now, we'll create an empty file to avoid breaking partner-chains-setup, but it will fail there.
        echo "" > "/shared/registered-${i}.utxo"
    else
        echo "$node_utxo" > "/shared/registered-${i}.utxo"
        echo "Saved UTXO for registered-$i: $node_utxo to /shared/registered-${i}.utxo"
    fi
done

echo "Querying and saving the first UTXO details for new address to /shared/genesis.utxo:"
if [ ${#utxo_list[@]} -gt 0 ]; then
    echo "${utxo_list[0]}" > /shared/genesis.utxo
    cat /shared/genesis.utxo > /runtime-values/genesis.utxo
    echo "Saved /shared/genesis.utxo: ${utxo_list[0]}"
    cat /shared/genesis.utxo
else
    echo "Error: No UTXOs found at $new_address to save for /shared/genesis.utxo"
fi

echo "Generating Mainchain Cold Keys for Registered Nodes..."
for i in {1..300}; do
    NODE_KEYS_DIR="/shared/node-keys/registered-${i}/keys"
    mkdir -p "$NODE_KEYS_DIR"
    echo "Generating cold keys for registered-$i in $NODE_KEYS_DIR..."
    cardano-cli node key-gen \
        --cold-verification-key-file "${NODE_KEYS_DIR}/cold.vkey" \
        --cold-signing-key-file "${NODE_KEYS_DIR}/cold.skey" \
        --operational-certificate-issue-counter-file "${NODE_KEYS_DIR}/cold.counter"
    if [ $? -eq 0 ]; then
        echo "Successfully generated cold keys for registered-$i."
    else
        echo "Error generating cold keys for registered-$i!"
        # Optionally exit here if this is critical
    fi
done

touch /shared/cardano.ready

wait
