#!/bin/bash

echo "Waiting for Cardano chain to start..."

while true; do
    if [ -f "/shared/cardano.ready" ]; then
        break
    else
        sleep 1
    fi
done

echo "Cardano chain ready. Starting Dolos..."

exec dolos daemon --config=/dolos.toml
