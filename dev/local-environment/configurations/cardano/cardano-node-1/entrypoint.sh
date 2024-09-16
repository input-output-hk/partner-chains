#! /bin/bash

chmod 600 /keys/*
chmod +x /busybox
chmod 777 /shared

echo "Calculating target time for synchronised chain start..."

remaining_seconds=$((60 - $(date +'%-S')))
if [ $remaining_seconds -le 20 ]; then
    sleep $remaining_seconds
fi

target_time=$(( ($(date +%s) / 60 + 1) * 60 ))
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

byron_hash=$(/bin/cardano-cli byron genesis print-genesis-hash --genesis-json /shared/byron/genesis.json)
shelley_hash=$(/bin/cardano-cli genesis hash --genesis /shared/shelley/genesis.json)
alonzo_hash=$(/bin/cardano-cli genesis hash --genesis /shared/shelley/genesis.alonzo.json)
conway_hash=$(/bin/cardano-cli genesis hash --genesis /shared/conway/genesis.conway.json)

/busybox sed "s/\"ByronGenesisHash\": \"[^\"]*\"/\"ByronGenesisHash\": \"$byron_hash\"/" /shared/node-1-config.json.base > /shared/node-1-config.json.base.byron
/busybox sed "s/\"ByronGenesisHash\": \"[^\"]*\"/\"ByronGenesisHash\": \"$byron_hash\"/" /shared/node-2-config.json.base > /shared/node-2-config.json.base.byron
/busybox sed "s/\"ByronGenesisHash\": \"[^\"]*\"/\"ByronGenesisHash\": \"$byron_hash\"/" /shared/node-3-config.json.base > /shared/node-3-config.json.base.byron
/busybox sed "s/\"ByronGenesisHash\": \"[^\"]*\"/\"ByronGenesisHash\": \"$byron_hash\"/" /shared/db-sync-config.json.base > /shared/db-sync-config.json.base.byron
/busybox sed "s/\"ShelleyGenesisHash\": \"[^\"]*\"/\"ShelleyGenesisHash\": \"$shelley_hash\"/" /shared/node-1-config.json.base.byron > /shared/node-1-config.base.shelley
/busybox sed "s/\"ShelleyGenesisHash\": \"[^\"]*\"/\"ShelleyGenesisHash\": \"$shelley_hash\"/" /shared/node-2-config.json.base.byron > /shared/node-2-config.base.shelley
/busybox sed "s/\"ShelleyGenesisHash\": \"[^\"]*\"/\"ShelleyGenesisHash\": \"$shelley_hash\"/" /shared/node-3-config.json.base.byron > /shared/node-3-config.base.shelley
/busybox sed "s/\"ShelleyGenesisHash\": \"[^\"]*\"/\"ShelleyGenesisHash\": \"$shelley_hash\"/" /shared/db-sync-config.json.base.byron > /shared/db-sync-config.base.shelley
/busybox sed "s/\"AlonzoGenesisHash\": \"[^\"]*\"/\"AlonzoGenesisHash\": \"$alonzo_hash\"/" /shared/node-1-config.base.shelley > /shared/node-1-config.json.base.conway
/busybox sed "s/\"AlonzoGenesisHash\": \"[^\"]*\"/\"AlonzoGenesisHash\": \"$alonzo_hash\"/" /shared/node-2-config.base.shelley > /shared/node-2-config.json.base.conway
/busybox sed "s/\"AlonzoGenesisHash\": \"[^\"]*\"/\"AlonzoGenesisHash\": \"$alonzo_hash\"/" /shared/node-3-config.base.shelley > /shared/node-3-config.json.base.conway
/busybox sed "s/\"AlonzoGenesisHash\": \"[^\"]*\"/\"AlonzoGenesisHash\": \"$alonzo_hash\"/" /shared/db-sync-config.base.shelley > /shared/db-sync-config.json.base.conway
/busybox sed "s/\"ConwayGenesisHash\": \"[^\"]*\"/\"ConwayGenesisHash\": \"$conway_hash\"/" /shared/node-1-config.json.base.conway > /shared/node-1-config.json
/busybox sed "s/\"ConwayGenesisHash\": \"[^\"]*\"/\"ConwayGenesisHash\": \"$conway_hash\"/" /shared/node-2-config.json.base.conway > /shared/node-2-config.json
/busybox sed "s/\"ConwayGenesisHash\": \"[^\"]*\"/\"ConwayGenesisHash\": \"$conway_hash\"/" /shared/node-3-config.json.base.conway > /shared/node-3-config.json
/busybox sed "s/\"ConwayGenesisHash\": \"[^\"]*\"/\"ConwayGenesisHash\": \"$conway_hash\"/" /shared/db-sync-config.json.base.conway > /shared/db-sync-config.json

echo "Updated ByronGenesisHash value in config files to: $byron_hash"
echo "Updated ShelleyGenesisHash value in config files to: $shelley_hash"
echo "Updated ConwayGenesisHash value in config files to: $conway_hash"

byron_startTimeMillis=$(($byron_startTime * 1000))
echo $byron_startTimeMillis > /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS
echo "Created /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS with value: $byron_startTimeMillis"

adjusted_target_time=$((target_time - 10))
current_epoch=$(date +%s%3N)
sleep_milliseconds=$((adjusted_target_time * 1000 - current_epoch))
sleep_seconds=$((sleep_milliseconds / 1000))
remaining_milliseconds=$((sleep_milliseconds % 1000))
total_sleep_time=$(printf "%.3f" "$(echo "$sleep_milliseconds / 1000" | /busybox bc)")
echo "Waiting for $total_sleep_time seconds until 10 seconds before the target time..."
sleep $total_sleep_time
echo "Current time is now: $(date +"%H:%M:%S.%3N"). Starting node..."

echo "Starting node..."
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

touch /shared/cardano-node-1.ready

echo "Waiting for node 2 and node 3 to start..."

while true; do
    if [ -f "/shared/cardano-node-2.ready" ] && [ -f "/shared/cardano-node-3.ready" ]; then
        break
    else
        sleep 10
    fi
done

echo "Generating new address and funding it with 2 UTXOs from the genesis address"

new_address=$(cardano-cli address build \
  --payment-verification-key-file /keys/funded_address.vkey \
  --testnet-magic 42)

echo "New address created: $new_address"

dave_address="addr_test1vphpcf32drhhznv6rqmrmgpuwq06kug0lkg22ux777rtlqst2er0r"
eve_address="addr_test1vzzt5pwz3pum9xdgxalxyy52m3aqur0n43pcl727l37ggscl8h7v8"

# Define the UTXO details and amounts
tx_in1="781cb948a37c7c38b43872af9b1e22135a94826eafd3740260a6db0a303885d8#0"
tx_in_amount=29993040000000000

# Define output amounts
tx_out1=1000000000 # new_address utxo 1
tx_out2=1000000000 # new_address utxo 2
tx_out3=1000000000 # partner-chains-node-4 (dave)
tx_out4=1000000000 # partner-chains-node-5 (eve)

# Total output without fee
total_output=$((tx_out1 + tx_out2 + tx_out3 + tx_out4))

fee=1000000

# Calculate remaining balance to return to the genesis address
change=$((tx_in_amount - total_output - fee))

# Build the raw transaction
cardano-cli transaction build-raw \
  --tx-in $tx_in1 \
  --tx-out "$new_address+$tx_out1" \
  --tx-out "$new_address+$tx_out2" \
  --tx-out "$dave_address+$tx_out3" \
  --tx-out "$eve_address+$tx_out4" \
  --tx-out "$new_address+$change" \
  --fee $fee \
  --out-file /data/tx.raw

# Sign the transaction
cardano-cli transaction sign \
  --tx-body-file /data/tx.raw \
  --signing-key-file /shared/shelley/genesis-utxo.skey \
  --testnet-magic 42 \
  --out-file /data/tx.signed

cat /data/tx.signed

echo "Transaction prepared, waiting 20 seconds for other nodes to start..."
sleep 20

echo "Submitting transaction..."
cardano-cli transaction submit \
  --tx-file /data/tx.signed \
  --testnet-magic 42

echo "Transaction submitted to fund registered candidates and governance authority. Waiting 40 seconds for transaction to process..."
sleep 40
echo "Balance:"

# Query UTXOs at new_address, dave_address, and eve_address
echo "Querying UTXO for new_address:"
cardano-cli query utxo \
  --testnet-magic 42 \
  --address $new_address

echo "Querying UTXO for Dave address:"
cardano-cli query utxo \
  --testnet-magic 42 \
  --address $dave_address

echo "Querying UTXO for Eve address:"
cardano-cli query utxo \
  --testnet-magic 42 \
  --address $eve_address

# Save dynamic values to shared config volume for other nodes to use
echo $new_address > /shared/FUNDED_ADDRESS
echo "Created /shared/FUNDED_ADDRESS with value: $new_address"

echo "Querying and saving the first UTXO details for Dave address to /shared/dave.utxo:"
cardano-cli query utxo --testnet-magic 42 --address "${dave_address}" | /busybox awk 'NR>2 { print $1 "#" $2; exit }' > /shared/dave.utxo
echo "UTXO details for Dave saved in /shared/dave.utxo."
cat /shared/dave.utxo

echo "Querying and saving the first UTXO details for Eve address to /shared/eve.utxo:"
cardano-cli query utxo --testnet-magic 42 --address "${eve_address}" | /busybox awk 'NR>2 { print $1 "#" $2; exit }' > /shared/eve.utxo
echo "UTXO details for Eve saved in /shared/eve.utxo."
cat /shared/eve.utxo

echo "Saving NATIVE_TOKEN_POLICY_ID, NATIVE_TOKEN_ASSET_NAME, and ILLIQUID_SUPPLY_VALIDATOR_ADDRESS to /shared:"
echo 'ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4' > /shared/NATIVE_TOKEN_POLICY_ID
echo '5043546f6b656e44656d6f' > /shared/NATIVE_TOKEN_ASSET_NAME
echo 'addr_test1wrhvtvx3f0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9rjdvz' > /shared/ILLIQUID_SUPPLY_VALIDATOR_ADDRESS 

touch /shared/cardano.ready

wait
