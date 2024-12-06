#!/bin/bash

echo "Waiting for Cardano chain to start..."

while true; do
    if [ -f "/shared/cardano.ready" ]; then
        break
    else
        sleep 10
    fi
done

echo "Cardano chain ready. Starting Ogmios..."

exec /bin/ogmios \
  --host=0.0.0.0 \
  --node-config=/shared/node-1-config.json \
  --node-socket=/node-ipc/node.socket &

wait
