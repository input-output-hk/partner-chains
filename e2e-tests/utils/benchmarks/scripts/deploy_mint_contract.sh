#!/bin/bash
# Deploys a mint contract and mints shielded tokens on a local Midnight node.
# Requires: local node running on ws://localhost:9944, built toolkit and toolkit-js.
set -euo pipefail

# --- Configuration ---
SEED="0000000000000000000000000000000000000000000000000000000000000101"
NONCE="2339000000001000000000000000000000000000000000000000000000000000" # Different nonce per contract
DOMAIN_SEP="beeb000000010000000000000000000000000000000000000000000000000000" # Token type
AMOUNT="1000"
NETWORK="undeployed"
NODE_URL="ws://henry.node.sc.iog.io:9944"
WORK_DIR="/home/christos/temp/mint-shielded-tokens-contract4"
TOOLKIT="./target/release/midnight-node-toolkit"
TOOLKIT_JS_DIR="./util/toolkit-js"

mkdir -p $WORK_DIR

# Derive wallet addresses from seed
COIN_PUBLIC=$($TOOLKIT show-address --network $NETWORK --seed $SEED --coin-public)
SHIELDED_DEST=$($TOOLKIT show-address --network $NETWORK --seed $SEED --shielded)

# --- Build toolkit-js ---
cd $TOOLKIT_JS_DIR
npm install
npm run build
npm run build-compact
npx fetch-compactc
npm run compact-mint
echo "1"
#--- Deploy mint contract ---
# Generate deployment intent with initial contract state
./dist/bin.js deploy \
    -c ./mint/mint.config.ts \
    --network $NETWORK \
    --coin-public "$COIN_PUBLIC" \
    --output $WORK_DIR/deploy.bin \
    --output-ps $WORK_DIR/private_state.json \
    --output-zswap $WORK_DIR/deploy_zswap.json

cd -

echo "2"
# Convert intent to transaction format
$TOOLKIT send-intent \
    --src-url $NODE_URL \
    --intent-file $WORK_DIR/deploy.bin \
    --compiled-contract-dir $TOOLKIT_JS_DIR/mint/out \
    --to-bytes \
    --dest-file $WORK_DIR/deploy_tx.mn
echo "3"
# Extract contract address from deploy transaction
CONTRACT_ADDRESS=$($TOOLKIT contract-address --src-file $WORK_DIR/deploy_tx.mn)
echo $CONTRACT_ADDRESS

# Submit deploy transaction to the node
$TOOLKIT generate-txs \
    --src-file $WORK_DIR/deploy_tx.mn \
    -r 1 send -d $NODE_URL
echo "4"
# --- Mint shielded tokens ---
# Fetch deployed contract state from chain
$TOOLKIT contract-state \
    --contract-address $CONTRACT_ADDRESS \
    -d $WORK_DIR/contract_state.mn \
    -s $NODE_URL

cd $TOOLKIT_JS_DIR
echo "5"
# Generate mint circuit proof
./dist/bin.js circuit \
    -c ./mint/mint.config.ts \
    --network $NETWORK \
    --input $WORK_DIR/contract_state.mn \
    --input-ps $WORK_DIR/private_state.json \
    --output $WORK_DIR/mint.bin \
    --output-ps $WORK_DIR/private_state2.json \
    --output-zswap $WORK_DIR/mint_zswap.json \
    --coin-public "$COIN_PUBLIC" \
    "$CONTRACT_ADDRESS" mint "$NONCE" "$DOMAIN_SEP" "$AMOUNT"

cd -
echo "6"
# Submit mint transaction, sending tokens to shielded destination
$TOOLKIT send-intent \
    --intent-file $WORK_DIR/mint.bin \
    --zswap-state-file $WORK_DIR/mint_zswap.json \
    --compiled-contract-dir $TOOLKIT_JS_DIR/mint/out \
    --shielded-destination "$SHIELDED_DEST" \
    -s $NODE_URL \
    -d $NODE_URL
echo "7"
# --- Display results ---
TOKEN_TYPE=$($TOOLKIT show-token-type \
    --contract-address $CONTRACT_ADDRESS \
    --domain-sep $DOMAIN_SEP \
    --shielded)

echo "Contract: $CONTRACT_ADDRESS"
echo "Token type: $TOKEN_TYPE"
echo "Wallet contents:"
$TOOLKIT show-wallet --seed $SEED -s $NODE_URL
