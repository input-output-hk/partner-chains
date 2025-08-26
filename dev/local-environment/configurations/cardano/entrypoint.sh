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

node4_address="addr_test1vphpcf32drhhznv6rqmrmgpuwq06kug0lkg22ux777rtlqst2er0r"
node5_address="addr_test1vzzt5pwz3pum9xdgxalxyy52m3aqur0n43pcl727l37ggscl8h7v8"
# An address that will keep an UTXO with script of a test V-function, related to the SPO rewards. See v-function.script file.
vfunction_address="addr_test1vzuasm5nqzh7n909f7wang7apjprpg29l2f9sk6shlt84rqep6nyc"

# Define the UTXO details and amounts
tx_in1="781cb948a37c7c38b43872af9b1e22135a94826eafd3740260a6db0a303885d8#0"
tx_in_amount=29993040000000000

# Define output amounts
tx_out1=1000000000 # new_address utxo 1
tx_out2=1000000000 # new_address utxo 2
tx_out3=1000000000 # partner-chains-node-4 (node-4)
tx_out4=1000000000 # partner-chains-node-5 (node-5)
tx_out5_lovelace=10000000
tx_out5_reward_token="1000000 $reward_token_policy_id.$reward_token_asset_name"
tx_out6=10000000

# Total output without fee
total_output=$((tx_out1 + tx_out2 + tx_out3 + tx_out4 + tx_out5_lovelace + tx_out6))

fee=1000000

# Calculate remaining balance to return to the genesis address
change=$((tx_in_amount - total_output - fee))

# Build the raw transaction
cardano-cli latest transaction build-raw \
  --tx-in $tx_in1 \
  --tx-out "$new_address+$tx_out1" \
  --tx-out "$new_address+$tx_out2" \
  --tx-out "$node4_address+$tx_out3" \
  --tx-out "$node5_address+$tx_out4" \
  --tx-out "$new_address+$change" \
  --tx-out "$new_address+$tx_out5_lovelace+$tx_out5_reward_token" \
  --tx-out "$vfunction_address+$tx_out6" \
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

echo "Transaction submitted to fund registered candidates and governance authority. Waiting 20 seconds for transaction to process..."
sleep 20
echo "Balance:"

# Query UTXOs at new_address, node4_address, and node5_address
echo "Querying UTXO for new_address:"
cardano-cli latest query utxo \
  --testnet-magic 42 \
  --address $new_address

echo "Querying UTXO for 'node-4' address:"
cardano-cli latest query utxo \
  --testnet-magic 42 \
  --address $node4_address

echo "Querying UTXO for 'node-5' address:"
cardano-cli latest query utxo \
  --testnet-magic 42 \
  --address $node5_address

# Save dynamic values to shared config volume for other nodes to use
echo $new_address > /shared/FUNDED_ADDRESS
echo "Created /shared/FUNDED_ADDRESS with value: $new_address"

echo "Querying and saving the first UTXO details for node-4 address to /shared/node4.utxo:"
cardano-cli latest query utxo --testnet-magic 42 --address "${node4_address}" | jq -r 'keys|.[0]' > /shared/node4.utxo
echo "UTXO details for node-4 saved in /shared/node4.utxo."
cat /shared/node4.utxo

echo "Querying and saving the first UTXO details for node-5 address to /shared/node5.utxo:"
cardano-cli latest query utxo --testnet-magic 42 --address "${node5_address}" | jq -r 'keys|.[0]' > /shared/node5.utxo
echo "UTXO details for node-5 saved in /shared/node5.utxo."
cat /shared/node5.utxo

echo "Querying and saving the first UTXO details for new address to /shared/genesis.utxo:"
cardano-cli latest query utxo --testnet-magic 42 --address "${new_address}" | jq -r 'keys|.[0]' > /shared/genesis.utxo
cat /shared/genesis.utxo > /runtime-values/genesis.utxo
cat /shared/genesis.utxo

touch /shared/cardano.ready

wait
