#!/bin/bash

# Check ACCOUNT and execute the corresponding command
case "$ACCOUNT" in
  alice)
    echo 'Waiting for Cardano chain to sync and pc-contracts-cli configuration to complete...'
    while true; do
        if [ -f "/shared/2-epochs.ready" ]; then
            break
        else
            sleep 10
        fi
    done
    export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)
    echo "MC__FIRST_EPOCH_TIMESTAMP_MILLIS=$MC__FIRST_EPOCH_TIMESTAMP_MILLIS"
    export COMMITTEE_CANDIDATE_ADDRESS=$(cat /shared/COMMITTEE_CANDIDATE_ADDRESS)
    echo "COMMITTEE_CANDIDATE_ADDRESS=$COMMITTEE_CANDIDATE_ADDRESS"
    export D_PARAMETER_POLICY_ID=$(cat /shared/D_PARAMETER_POLICY_ID)
    echo "D_PARAMETER_POLICY_ID=$D_PARAMETER_POLICY_ID"
    export PERMISSIONED_CANDIDATES_POLICY_ID=$(cat /shared/PERMISSIONED_CANDIDATES_POLICY_ID)
    echo "PERMISSIONED_CANDIDATES_POLICY_ID=$PERMISSIONED_CANDIDATES_POLICY_ID"
    export NATIVE_TOKEN_POLICY_ID=$(cat /shared/NATIVE_TOKEN_POLICY_ID)
    echo "NATIVE_TOKEN_POLICY_ID=$NATIVE_TOKEN_POLICY_ID"
    export NATIVE_TOKEN_ASSET_NAME=$(cat /shared/NATIVE_TOKEN_ASSET_NAME)
    echo "NATIVE_TOKEN_ASSET_NAME=$NATIVE_TOKEN_ASSET_NAME"
    export ILLIQUID_SUPPLY_VALIDATOR_ADDRESS=$(cat /shared/ILLIQUID_SUPPLY_VALIDATOR_ADDRESS)
    echo "ILLIQUID_SUPPLY_VALIDATOR_ADDRESS=$ILLIQUID_SUPPLY_VALIDATOR_ADDRESS"
    echo "pc-contracts-cli configuration complete. Starting node..."
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
    export MC__FIRST_EPOCH_TIMESTAMP_MILLIS="1732616340000"
    export COMMITTEE_CANDIDATE_ADDRESS="COMMITTEE_CANDIDATE_ADDRESS=addr_test1wre9jl2m00fwdsj5p4tdu37rcnm6eufce8lsvmpjl2ep2qqj92rxy"
    export D_PARAMETER_POLICY_ID="D_PARAMETER_POLICY_ID=dd7628ddd457d57ce48ac8e3046bef4b5d2fb650bc2366812465deb2"
    export PERMISSIONED_CANDIDATES_POLICY_ID="PERMISSIONED_CANDIDATES_POLICY_ID=15e98f9aeaecf4c0baf2fedac76b3a0f51d7e87f25dfffe723184592"
    export NATIVE_TOKEN_POLICY_ID="ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4"
    export NATIVE_TOKEN_ASSET_NAME="5043546f6b656e44656d6f"
    export ILLIQUID_SUPPLY_VALIDATOR_ADDRESS="addr_test1wrhvtvx3f0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9rjdvz"
    export SIDECHAIN_BLOCK_BENEFICIARY="0000000000000000000000000000000000000000000000000000000000000002"
    echo "Starting node..."
    /usr/local/bin/partner-chains-node \
      --bob \
      --chain=/chain-spec.json \
      --node-key=0000000000000000000000000000000000000000000000000000000000000002 \
      --bootnodes="/ip4/$ALICE_IP/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
      --base-path=/data \
      --unsafe-rpc-external \
      --rpc-port=9933 \
      --rpc-cors=all \
      --prometheus-port=9615 \
      --prometheus-external \
      --state-pruning=archive \
      --blocks-pruning=archive &
    ;;
  charlie)
    export MC__FIRST_EPOCH_TIMESTAMP_MILLIS="1732616340000"
    export COMMITTEE_CANDIDATE_ADDRESS="COMMITTEE_CANDIDATE_ADDRESS=addr_test1wre9jl2m00fwdsj5p4tdu37rcnm6eufce8lsvmpjl2ep2qqj92rxy"
    export D_PARAMETER_POLICY_ID="D_PARAMETER_POLICY_ID=dd7628ddd457d57ce48ac8e3046bef4b5d2fb650bc2366812465deb2"
    export PERMISSIONED_CANDIDATES_POLICY_ID="PERMISSIONED_CANDIDATES_POLICY_ID=15e98f9aeaecf4c0baf2fedac76b3a0f51d7e87f25dfffe723184592"
    export NATIVE_TOKEN_POLICY_ID="ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4"
    export NATIVE_TOKEN_ASSET_NAME="5043546f6b656e44656d6f"
    export ILLIQUID_SUPPLY_VALIDATOR_ADDRESS="addr_test1wrhvtvx3f0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9rjdvz"
    export SIDECHAIN_BLOCK_BENEFICIARY="0000000000000000000000000000000000000000000000000000000000000003"
    echo "Starting node..."
    /usr/local/bin/partner-chains-node \
      --charlie \
      --chain=/chain-spec.json \
      --node-key=0000000000000000000000000000000000000000000000000000000000000003 \
      --bootnodes="/ip4/$ALICE_IP/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
      --base-path=/data \
      --unsafe-rpc-external \
      --rpc-port=9933 \
      --rpc-cors=all \
      --prometheus-port=9615 \
      --prometheus-external \
      --state-pruning=archive \
      --blocks-pruning=archive &
    ;;
  dave)
    export MC__FIRST_EPOCH_TIMESTAMP_MILLIS="1732616340000"
    export COMMITTEE_CANDIDATE_ADDRESS="COMMITTEE_CANDIDATE_ADDRESS=addr_test1wre9jl2m00fwdsj5p4tdu37rcnm6eufce8lsvmpjl2ep2qqj92rxy"
    export D_PARAMETER_POLICY_ID="D_PARAMETER_POLICY_ID=dd7628ddd457d57ce48ac8e3046bef4b5d2fb650bc2366812465deb2"
    export PERMISSIONED_CANDIDATES_POLICY_ID="PERMISSIONED_CANDIDATES_POLICY_ID=15e98f9aeaecf4c0baf2fedac76b3a0f51d7e87f25dfffe723184592"
    export NATIVE_TOKEN_POLICY_ID="ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4"
    export NATIVE_TOKEN_ASSET_NAME="5043546f6b656e44656d6f"
    export ILLIQUID_SUPPLY_VALIDATOR_ADDRESS="addr_test1wrhvtvx3f0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9rjdvz"
    export SIDECHAIN_BLOCK_BENEFICIARY="0000000000000000000000000000000000000000000000000000000000000004"
    echo "Starting node..."
    /usr/local/bin/partner-chains-node \
      --chain=/chain-spec.json \
      --dave \
      --node-key=0000000000000000000000000000000000000000000000000000000000000004 \
      --bootnodes="/ip4/$ALICE_IP/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
      --base-path=/data \
      --unsafe-rpc-external \
      --rpc-port=9933 \
      --rpc-cors=all \
      --prometheus-port=9615 \
      --prometheus-external \
      --state-pruning=archive \
      --blocks-pruning=archive &
    ;;
  eve)
    export MC__FIRST_EPOCH_TIMESTAMP_MILLIS="1732616340000"
    export COMMITTEE_CANDIDATE_ADDRESS="COMMITTEE_CANDIDATE_ADDRESS=addr_test1wre9jl2m00fwdsj5p4tdu37rcnm6eufce8lsvmpjl2ep2qqj92rxy"
    export D_PARAMETER_POLICY_ID="D_PARAMETER_POLICY_ID=dd7628ddd457d57ce48ac8e3046bef4b5d2fb650bc2366812465deb2"
    export PERMISSIONED_CANDIDATES_POLICY_ID="PERMISSIONED_CANDIDATES_POLICY_ID=15e98f9aeaecf4c0baf2fedac76b3a0f51d7e87f25dfffe723184592"
    export NATIVE_TOKEN_POLICY_ID="ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4"
    export NATIVE_TOKEN_ASSET_NAME="5043546f6b656e44656d6f"
    export ILLIQUID_SUPPLY_VALIDATOR_ADDRESS="addr_test1wrhvtvx3f0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9rjdvz"
    export SIDECHAIN_BLOCK_BENEFICIARY="0000000000000000000000000000000000000000000000000000000000000005"
    echo "Starting node..."
    /usr/local/bin/partner-chains-node \
      --chain=/chain-spec.json \
      --validator \
      --node-key=0000000000000000000000000000000000000000000000000000000000000005 \
      --bootnodes="/ip4/$ALICE_IP/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
      --base-path=/data \
      --keystore-path=/keystore \
      --unsafe-rpc-external \
      --rpc-port=9933 \
      --rpc-cors=all \
      --prometheus-port=9615 \
      --prometheus-external \
      --state-pruning=archive \
      --blocks-pruning=archive &
    ;;
  ferdie)
    export MC__FIRST_EPOCH_TIMESTAMP_MILLIS="1732616340000"
    export COMMITTEE_CANDIDATE_ADDRESS="COMMITTEE_CANDIDATE_ADDRESS=addr_test1wre9jl2m00fwdsj5p4tdu37rcnm6eufce8lsvmpjl2ep2qqj92rxy"
    export D_PARAMETER_POLICY_ID="D_PARAMETER_POLICY_ID=dd7628ddd457d57ce48ac8e3046bef4b5d2fb650bc2366812465deb2"
    export PERMISSIONED_CANDIDATES_POLICY_ID="PERMISSIONED_CANDIDATES_POLICY_ID=15e98f9aeaecf4c0baf2fedac76b3a0f51d7e87f25dfffe723184592"
    export NATIVE_TOKEN_POLICY_ID="ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4"
    export NATIVE_TOKEN_ASSET_NAME="5043546f6b656e44656d6f"
    export ILLIQUID_SUPPLY_VALIDATOR_ADDRESS="addr_test1wrhvtvx3f0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9rjdvz"
    export SIDECHAIN_BLOCK_BENEFICIARY="0000000000000000000000000000000000000000000000000000000000000006"
    echo "Starting node..."
    /usr/local/bin/partner-chains-node \
      --chain=/chain-spec.json \
      --validator \
      --node-key=0000000000000000000000000000000000000000000000000000000000000006 \
      --bootnodes="/ip4/$ALICE_IP/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
      --base-path=/data \
      --keystore-path=/keystore \
      --unsafe-rpc-external \
      --rpc-port=9933 \
      --rpc-cors=all \
      --prometheus-port=9615 \
      --prometheus-external \
      --state-pruning=archive \
      --blocks-pruning=archive &
    ;;
  *)
    echo "Error: Unknown ACCOUNT value '$ACCOUNT'. Please set ACCOUNT to 'alice', 'bob', 'charlie', 'dave', 'eve', or 'ferdie' in your .envrc."
    exit 1
    ;;
esac

wait
