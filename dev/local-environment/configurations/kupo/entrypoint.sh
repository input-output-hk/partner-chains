#!/bin/bash

echo "Waiting for Cardano chain to start..."

while true; do
    if [ -f "/shared/cardano.ready" ]; then
        break
    else
        sleep 10
    fi
done

echo "Cardano chain ready. Starting Kupo..."

exec /bin/kupo \
  --node-socket=/node-ipc/node.socket \
  --node-config=/shared/node-1-config.json \
  --host=0.0.0.0 \
  --workdir=/kupo-workdir \
  --match=* \
  --since=origin &

wait
