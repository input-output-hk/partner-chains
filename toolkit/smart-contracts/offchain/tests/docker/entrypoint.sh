#! /bin/bash

chmod 400 /config/keys/kes.skey
chmod 400 /config/keys/vrf.skey

byron_startTime=$(( $(date +%s) + 1 ))
shelley_systemStart=$(date --utc +"%Y-%m-%dT%H:%M:%SZ" --date="@$byron_startTime")

/busybox sed "s/\"startTime\": [0-9]*/\"startTime\": $byron_startTime/" /config/genesis/byron.base.json > /config/genesis/byron.json
/busybox sed "s/\"systemStart\": \"[^\"]*\"/\"systemStart\": \"$shelley_systemStart\"/" /config/genesis/shelley.base.json > /config/genesis/shelley.json

byron_hash=$(/bin/cardano-cli byron genesis print-genesis-hash --genesis-json /config/genesis/byron.json)
shelley_hash=$(/bin/cardano-cli latest genesis hash --genesis /config/genesis/shelley.json)
alonzo_hash=$(/bin/cardano-cli latest genesis hash --genesis /config/genesis/alonzo.json)
conway_hash=$(/bin/cardano-cli latest genesis hash --genesis /config/genesis/conway.json)

/busybox sed "s/\"ByronGenesisHash\": \"[^\"]*\"/\"ByronGenesisHash\": \"$byron_hash\"/" /config/config.base.json > /config/config.byron_hash.json
/busybox sed "s/\"ShelleyGenesisHash\": \"[^\"]*\"/\"ShelleyGenesisHash\": \"$shelley_hash\"/" /config/config.byron_hash.json > /config/config.shelley_hash.json
/busybox sed "s/\"AlonzoGenesisHash\": \"[^\"]*\"/\"AlonzoGenesisHash\": \"$alonzo_hash\"/" /config/config.shelley_hash.json > /config/config.alonzo_hash.json
/busybox sed "s/\"ConwayGenesisHash\": \"[^\"]*\"/\"ConwayGenesisHash\": \"$conway_hash\"/" /config/config.alonzo_hash.json > /config/config.json

echo "Starting ogmios..."

ogmios \
  --host 0.0.0.0 \
  --node-config /config/config.json \
  --node-socket /ipc/node.socket &
ogmios_status=$?

if [ $ogmios_status -ne 0 ]; then
  echo "Failed to start ogmios: $ogmios_status"
  exit $ogmios_status
fi

echo "Starting cardano-node..."

cardano-node run \
  --topology /config/topology.json \
  --database-path /db \
  --port 3000 \
  --host-addr 0.0.0.0 \
  --config /config/config.json \
  --shelley-kes-key /config/keys/kes.skey \
  --shelley-vrf-key /config/keys/vrf.skey \
  --shelley-operational-certificate /config/keys/node.cert \
  --socket-path /ipc/node.socket
