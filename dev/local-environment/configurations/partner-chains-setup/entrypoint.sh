#!/bin/bash

apt -qq update &> /dev/null
apt -qq -y install curl jq ncat &> /dev/null

cp /usr/local/bin/partner-chains-node /partner-chains-node
echo "Using Partner Chains node version:"
./partner-chains-node --version

echo "Waiting for the Cardano network to sync and for Ogmios to start..."

while true; do
    if nc -z ogmios $OGMIOS_PORT; then
        break
    else
        sleep 1
    fi
done

echo "Beginning configuration..."

chmod 644 /shared/shelley/genesis-utxo.skey

echo "Initializing governance authority ..."

export GENESIS_UTXO=$(cat /shared/genesis.utxo)

./partner-chains-node smart-contracts governance init \
    --ogmios-url http://ogmios:$OGMIOS_PORT \
    --genesis-utxo $GENESIS_UTXO \
    --payment-key-file /keys/funded_address.skey \
    --governance-authority $GOVERNANCE_AUTHORITY \
    --threshold 1

if [ $? -eq 0 ]; then
   echo "Successfully initialized governance authority!"
else
    echo "Failed to initialize governance authority!"
    exit 1
fi

echo "Generating addresses.json file..."

./partner-chains-node smart-contracts get-scripts \
    --ogmios-url http://ogmios:$OGMIOS_PORT \
    --genesis-utxo $GENESIS_UTXO \
> addresses.json

export COMMITTEE_CANDIDATE_ADDRESS=$(jq -r '.addresses.CommitteeCandidateValidator' addresses.json)
echo "Committee candidate address: $COMMITTEE_CANDIDATE_ADDRESS"

export D_PARAMETER_POLICY_ID=$(jq -r '.policyIds.DParameter' addresses.json)
echo "D parameter policy ID: $D_PARAMETER_POLICY_ID"

export PERMISSIONED_CANDIDATES_POLICY_ID=$(jq -r '.policyIds.PermissionedCandidates' addresses.json)
echo "Permissioned candidates policy ID: $PERMISSIONED_CANDIDATES_POLICY_ID"

echo "Setting values for NATIVE_TOKEN_POLICY_ID, NATIVE_TOKEN_ASSET_NAME, and ILLIQUID_SUPPLY_VALIDATOR_ADDRESS for chain-spec creation"
export NATIVE_TOKEN_POLICY_ID="1fab25f376bc49a181d03a869ee8eaa3157a3a3d242a619ca7995b2b"
export NATIVE_TOKEN_ASSET_NAME="52657761726420746f6b656e"
export ILLIQUID_SUPPLY_VALIDATOR_ADDRESS="addr_test1wpy8ewg646rg4ce78nl3aassmkquf4wlxcaugqlxwzcylkca0q8v3"

echo "Inserting D parameter..."

./partner-chains-node smart-contracts upsert-d-parameter \
    --ogmios-url http://ogmios:$OGMIOS_PORT \
    --genesis-utxo $GENESIS_UTXO \
    --permissioned-candidates-count 10 \
    --registered-candidates-count 300 \
    --payment-key-file /keys/funded_address.skey

if [ $? -eq 0 ]; then
    echo "Successfully inserted D-parameter (P = 10, R = 300)!"
else
    echo "Couldn't insert D-parameter..."
    exit 1
fi

# Generate and insert permissioned candidates (1-10)
echo "Generating and inserting permissioned candidates..."
> permissioned_candidates.csv

for i in {1..10}; do
    node_name="permissioned-$i"
    echo "Processing $node_name..."
    
    # Create directory for node keys
    mkdir -p /partner-chains-nodes/$node_name/keys
    
    # Generate keys for each node
    ./partner-chains-node key generate \
        --scheme ecdsa \
        --output-type json \
        --output-file /partner-chains-nodes/$node_name/keys/sidechain.json
    
    ./partner-chains-node key generate \
        --scheme sr25519 \
        --output-type json \
        --output-file /partner-chains-nodes/$node_name/keys/aura.json
    
    ./partner-chains-node key generate \
        --scheme ed25519 \
        --output-type json \
        --output-file /partner-chains-nodes/$node_name/keys/grandpa.json
    
    # Extract public keys
    sidechain_vkey=$(jq -r '.publicKey' /partner-chains-nodes/$node_name/keys/sidechain.json)
    aura_vkey=$(jq -r '.publicKey' /partner-chains-nodes/$node_name/keys/aura.json)
    grandpa_vkey=$(jq -r '.publicKey' /partner-chains-nodes/$node_name/keys/grandpa.json)
    
    # Add to permissioned candidates list
    echo "$sidechain_vkey:$aura_vkey:$grandpa_vkey" >> permissioned_candidates.csv
done

echo "Inserting permissioned candidates..."
./partner-chains-node smart-contracts upsert-permissioned-candidates \
    --ogmios-url http://ogmios:$OGMIOS_PORT \
    --genesis-utxo $GENESIS_UTXO \
    --permissioned-candidates-file permissioned_candidates.csv \
    --payment-key-file /keys/funded_address.skey

if [ $? -eq 0 ]; then
    echo "Permissioned candidates inserted successfully!"
else
    echo "Failed to insert permissioned candidates..."
    exit 1
fi

# Generate and register registered nodes (1-300)
echo "Generating and registering registered candidates..."
for i in {1..300}; do
    node_name="registered-$i"
    echo "Processing $node_name..."
    
    # Create directory for node keys
    mkdir -p /partner-chains-nodes/$node_name/keys
    
    # Generate keys for each node
    ./partner-chains-node key generate \
        --scheme ecdsa \
        --output-type json \
        --output-file /partner-chains-nodes/$node_name/keys/sidechain.json
    
    ./partner-chains-node key generate \
        --scheme sr25519 \
        --output-type json \
        --output-file /partner-chains-nodes/$node_name/keys/aura.json
    
    ./partner-chains-node key generate \
        --scheme ed25519 \
        --output-type json \
        --output-file /partner-chains-nodes/$node_name/keys/grandpa.json
    
    # Extract keys and generate signatures
    sidechain_signing_key=$(jq -r '.secretKey' /partner-chains-nodes/$node_name/keys/sidechain.json)
    
    # Process registration signatures
    registration_output=$(./partner-chains-node registration-signatures \
        --genesis-utxo $GENESIS_UTXO \
        --mainchain-signing-key /keys/cold.skey \
        --sidechain-signing-key $sidechain_signing_key \
        --registration-utxo $GENESIS_UTXO)
    
    # Extract signatures and keys
    spo_public_key=$(echo "$registration_output" | jq -r ".spo_public_key")
    spo_signature=$(echo "$registration_output" | jq -r ".spo_signature")
    sidechain_public_key=$(echo "$registration_output" | jq -r ".sidechain_public_key")
    sidechain_signature=$(echo "$registration_output" | jq -r ".sidechain_signature")
    aura_vkey=$(jq -r '.publicKey' /partner-chains-nodes/$node_name/keys/aura.json)
    grandpa_vkey=$(jq -r '.publicKey' /partner-chains-nodes/$node_name/keys/grandpa.json)
    
    # Register the node
    ./partner-chains-node smart-contracts register \
        --ogmios-url http://ogmios:$OGMIOS_PORT \
        --genesis-utxo $GENESIS_UTXO \
        --spo-public-key $spo_public_key \
        --spo-signature $spo_signature \
        --sidechain-public-keys $sidechain_public_key:$aura_vkey:$grandpa_vkey \
        --sidechain-signature $sidechain_signature \
        --registration-utxo $GENESIS_UTXO \
        --payment-key-file /keys/funded_address.skey
done

echo "Generating chain-spec.json file for Partnerchain Nodes..."
./partner-chains-node build-spec --disable-default-bootnode > chain-spec.json

echo "Configuring Initial Validators..."
# Generate initial validators array
echo "[" > initial_validators.json
for i in {1..10}; do
    node_name="permissioned-$i"
    sidechain_vkey=$(jq -r '.publicKey' /partner-chains-nodes/$node_name/keys/sidechain.json)
    aura_vkey=$(jq -r '.publicKey' /partner-chains-nodes/$node_name/keys/aura.json)
    grandpa_vkey=$(jq -r '.publicKey' /partner-chains-nodes/$node_name/keys/grandpa.json)
    
    if [ $i -gt 1 ]; then
        echo "," >> initial_validators.json
    fi
    
    cat <<EOF >> initial_validators.json
    [
        "$sidechain_vkey",
        {
            "aura": "$aura_vkey",
            "grandpa": "$grandpa_vkey"
        }
    ]
EOF
done
echo "]" >> initial_validators.json

# Update chain-spec.json with initial validators
jq --slurpfile validators initial_validators.json '.genesis.runtimeGenesis.config.session.initialValidators = $validators[0]' chain-spec.json > chain-spec.json.tmp
mv chain-spec.json.tmp chain-spec.json

cp chain-spec.json /shared/chain-spec.json
echo "chain-spec.json generation complete."

echo "Copying pc-chain-config.json file to /shared/pc-chain-config.json..."
cp pc-chain-config.json /shared/pc-chain-config.json

touch /shared/chain-spec.ready
touch /shared/partner-chains-setup.ready

echo "Setup complete!"

echo -e "\n===== Partnerchain Configuration Complete =====\n"
echo -e "Container will now idle, but will remain available for accessing the partner-chains-node commands as follows:\n"
echo "docker exec partner-chains-setup partner-chains-node smart-contracts --help"

echo "Waiting 2 epochs for DParam to become active..."
epoch=$(curl -s --request POST \
    --url "http://ogmios:1337" \
    --header 'Content-Type: application/json' \
    --data '{"jsonrpc": "2.0", "method": "queryLedgerState/epoch"}' | jq .result)
n_2_epoch=$((epoch + 2))
echo "Current epoch: $epoch"
while [ $epoch -lt $n_2_epoch ]; do
  sleep 10
  epoch=$(curl -s --request POST \
    --url "http://ogmios:1337" \
    --header 'Content-Type: application/json' \
    --data '{"jsonrpc": "2.0", "method": "queryLedgerState/epoch"}' | jq .result)
  echo "Current epoch: $epoch"
done
echo "DParam is now active!"

tail -f /dev/null
