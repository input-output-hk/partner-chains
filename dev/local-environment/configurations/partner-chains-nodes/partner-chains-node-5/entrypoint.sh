#!/bin/sh

echo 'Waiting for Cardano chain to sync and pc-contracts-cli configuraton to complete...'

while true; do
    if [ -f "/shared/pc-contracts-cli.ready" ]; then
        break
    else
        sleep 1
    fi
done

echo "pc-contracts-cli configuration complete. Waiting 2 epochs to start..."

while true; do
    if [ -f "/shared/2-epochs.ready" ]; then
        break
    else
        sleep 1
    fi
done

echo "2 mainchain epochs passed, starting node..."

while true; do
    if [ -f "/shared/partner-chains-node-1.ready" ] && [ -f "/shared/partner-chains-node-2.ready" ] && [ -f "/shared/partner-chains-node-3.ready" ] && [ -f "/shared/partner-chains-node-4.ready" ]; then
        break
    else
        sleep 1
    fi
done

export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)

/usr/local/bin/partner-chains-node \
  --chain=/shared/chain-spec.json \
  --validator \
  --node-key=0000000000000000000000000000000000000000000000000000000000000005 \
  --bootnodes="/dns/partner-chains-node-1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
  --base-path=/data \
  --keystore-path=/keystore \
  --unsafe-rpc-external \
  --rpc-port=9937 \
  --rpc-cors=all \
  --prometheus-port=9619 \
  --prometheus-external \
  --state-pruning=archive \
  --blocks-pruning=archive &

  touch /shared/partner-chains-node-5.ready

  wait
