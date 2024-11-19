#!/bin/sh

echo 'Waiting for Cardano chain to sync and pc-contracts-cli configuration to complete...'

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
export ILLIQUID_SUPPLY_VALIDATOR_ADDRESS=$(cat /shared/ILLIQUID_SUPPLY_VALIDATOR_ADDRESS)

# Check ACCOUNT and execute the corresponding command
case "$ACCOUNT" in
  alice)
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
    ;;
  bob)
    /usr/local/bin/partner-chains-node \
      --bob \
      --chain=/shared/chain-spec.json \
      --node-key=0000000000000000000000000000000000000000000000000000000000000002 \
      --bootnodes="/ip4/3.70.234.116/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
      --base-path=/data \
      --unsafe-rpc-external \
      --rpc-port=9934 \
      --rpc-cors=all \
      --prometheus-port=9616 \
      --prometheus-external \
      --state-pruning=archive \
      --blocks-pruning=archive &
    ;;
  charlie)
    /usr/local/bin/partner-chains-node \
      --charlie \
      --chain=/shared/chain-spec.json \
      --node-key=0000000000000000000000000000000000000000000000000000000000000003 \
      --bootnodes="/ip4/3.70.234.116/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
      --base-path=/data \
      --unsafe-rpc-external \
      --rpc-port=9935 \
      --rpc-cors=all \
      --prometheus-port=9617 \
      --prometheus-external \
      --state-pruning=archive \
      --blocks-pruning=archive &
    ;;
  dave)
    /usr/local/bin/partner-chains-node \
      --chain=/shared/chain-spec.json \
      --validator \
      --node-key=0000000000000000000000000000000000000000000000000000000000000004 \
      --bootnodes="/ip4/3.70.234.116/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
      --base-path=/data \
      --keystore-path=/keystore \
      --unsafe-rpc-external \
      --rpc-port=9936 \
      --rpc-cors=all \
      --prometheus-port=9618 \
      --prometheus-external \
      --state-pruning=archive \
      --blocks-pruning=archive &
    ;;
  eve)
    /usr/local/bin/partner-chains-node \
      --chain=/shared/chain-spec.json \
      --validator \
      --node-key=0000000000000000000000000000000000000000000000000000000000000005 \
      --bootnodes="/ip4/3.70.234.116/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
      --base-path=/data \
      --keystore-path=/keystore \
      --unsafe-rpc-external \
      --rpc-port=9937 \
      --rpc-cors=all \
      --prometheus-port=9619 \
      --prometheus-external \
      --state-pruning=archive \
      --blocks-pruning=archive &
    ;;
  ferdie)
    /usr/local/bin/partner-chains-node \
      --chain=/shared/chain-spec.json \
      --validator \
      --node-key=0000000000000000000000000000000000000000000000000000000000000006 \
      --bootnodes="/ip4/3.70.234.116/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
      --base-path=/data \
      --keystore-path=/keystore \
      --unsafe-rpc-external \
      --rpc-port=9937 \
      --rpc-cors=all \
      --prometheus-port=9619 \
      --prometheus-external \
      --state-pruning=archive \
      --blocks-pruning=archive &
    ;;
  *)
    echo "Error: Unknown ACCOUNT value '$ACCOUNT'. Please set ACCOUNT to 'alice', 'bob', 'charlie', 'dave', 'eve', or 'ferdie' in your .envrc."
    exit 1
    ;;
esac

touch /shared/partner-chains-node-1.ready

wait
