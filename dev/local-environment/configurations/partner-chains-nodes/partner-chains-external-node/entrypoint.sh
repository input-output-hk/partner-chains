#!/bin/sh

echo 'Waiting for Cardano chain to sync and pc-contracts-cli configuraton to complete...'

while true; do
    if [ -f "/shared/pc-contracts-cli.ready" ]; then
        break
    else
        sleep 10
    fi
done

echo "pc-contracts-cli configuration complete. Starting node..."

export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)
export COMMITTEE_CANDIDATE_ADDRESS=$(cat /shared/COMMITTEE_CANDIDATE_ADDRESS)
export D_PARAMETER_POLICY_ID=$(cat /shared/D_PARAMETER_POLICY_ID)
export PERMISSIONED_CANDIDATES_POLICY_ID=$(cat /shared/PERMISSIONED_CANDIDATES_POLICY_ID)
export NATIVE_TOKEN_POLICY_ID=$(cat /shared/NATIVE_TOKEN_POLICY_ID)
export NATIVE_TOKEN_ASSET_NAME=$(cat /shared/NATIVE_TOKEN_ASSET_NAME)
export ILLIQUID_SUPPLY_VALIDATOR_ADDRESS==$(cat /shared/ILLIQUID_SUPPLY_VALIDATOR_ADDRESS)

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