#!/bin/sh

echo 'Waiting for Cardano chain to sync and pc-contracts-cli configuration to complete...'

while true; do
    if [ -f "/shared/pc-contracts-cli.ready" ]; then
        break
    else
        sleep 1
    fi
done

echo "pc-contracts-cli configuration complete. Starting node..."

export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)

/usr/local/bin/partner-chains-node \
  --alice \
  --chain=/shared/chain-spec.json \
  --node-key=0000000000000000000000000000000000000000000000000000000000000001 \
  --base-path=/data \
  --unsafe-rpc-external \
  --rpc-port=9933 \
  --rpc-cors=all \
  --prometheus-port=9615 \
  --prometheus-external \
  --state-pruning=archive \
  --blocks-pruning=archive &

  touch /shared/partner-chains-node-1.ready

  wait
