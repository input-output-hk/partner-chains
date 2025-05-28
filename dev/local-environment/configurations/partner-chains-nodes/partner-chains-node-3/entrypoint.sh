#!/bin/sh

echo 'Waiting for Cardano chain to sync and Partner Chains smart contracts setup to complete...'

while true; do
    if [ -f "/shared/partner-chains-setup.ready" ]; then
        break
    else
        sleep 1
    fi
done

echo "Partner Chains smart contracts setup complete. Starting node..."

export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)

/usr/local/bin/partner-chains-node \
  --charlie \
  --chain=/shared/chain-spec.json \
  --node-key=0000000000000000000000000000000000000000000000000000000000000003 \
  --bootnodes="/dns/partner-chains-node-1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
  --base-path=/data \
  --unsafe-rpc-external \
  --rpc-port=9935 \
  --rpc-cors=all \
  -lbeefy=trace \
  --prometheus-port=9617 \
  --prometheus-external \
  --state-pruning=archive \
  --blocks-pruning=archive &

  touch /shared/partner-chains-node-3.ready

  wait
