#!/bin/bash

apt -qq update &> /dev/null
apt -qq -y install curl jq ncat &> /dev/null

cp /usr/local/bin/partner-chains-node /partner-chains-node
echo "Using Partner Chains node version:"
./partner-chains-node --version

echo "Waiting for /shared/cardano.ready to signal that Cardano node setup is complete..."
while [ ! -f "/shared/cardano.ready" ]; do
    sleep 5
done
echo "Cardano node setup completed."

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
export ILLIQUID_SUPPLY_VALIDATOR_ADDRESS=$(jq -r '.addresses.IlliquidCirculationSupplyValidator' addresses.json)
echo "Illiquid Circulation Supply Validator address: $ILLIQUID_SUPPLY_VALIDATOR_ADDRESS"
export NATIVE_TOKEN_POLICY_ID="1fab25f376bc49a181d03a869ee8eaa3157a3a3d242a619ca7995b2b"
export NATIVE_TOKEN_ASSET_NAME="52657761726420746f6b656e"

export GOVERNED_MAP_VALIDATOR_ADDRESS=$(jq -r '.addresses.GovernedMapValidator' addresses.json)
echo "Governed Map Validator Address: $GOVERNED_MAP_VALIDATOR_ADDRESS"
export GOVERNED_MAP_POLICY_ID=$(jq -r '.policyIds.GovernedMap' addresses.json)
echo "Governed Map Policy ID: $GOVERNED_MAP_POLICY_ID"

echo "Inserting D parameter..."

./partner-chains-node smart-contracts upsert-d-parameter \
    --ogmios-url http://ogmios:$OGMIOS_PORT \
    --genesis-utxo $GENESIS_UTXO \
    --permissioned-candidates-count "$NUM_PERMISSIONED_NODES_TO_PROCESS" \
    --registered-candidates-count "$NUM_REGISTERED_NODES_TO_PROCESS" \
    --payment-key-file /keys/funded_address.skey

if [ $? -eq 0 ]; then
    echo "Successfully inserted D-parameter (P = $NUM_PERMISSIONED_NODES_TO_PROCESS, R = $NUM_REGISTERED_NODES_TO_PROCESS)!"
else
    echo "Couldn't insert D-parameter..."
    exit 1
fi

# Generate and insert permissioned candidates (1-10)
echo "Generating and inserting permissioned candidates..."
> permissioned_candidates.csv

for ((i=1; i<=NUM_PERMISSIONED_NODES_TO_PROCESS; i++)); do
    node_name="permissioned-$i"
    echo "Processing $node_name..."
    
    # Create directory for node keys
    mkdir -p /shared/node-keys/$node_name/keys
    mkdir -p /shared/node-keys/$node_name/keystore
    
    # Generate keys for each node
    echo "[DEBUG] Generating sidechain keys for $node_name..."
    ./partner-chains-node key generate \
        --scheme ecdsa \
        --output-type json \
        > /shared/node-keys/$node_name/keys/sidechain.json
    
    if [ $? -ne 0 ]; then
        echo "[ERROR] partner-chains-node key generate (ecdsa) failed for $node_name!"
        # Optionally exit or handle error
    fi
    if [ "$i" -eq 1 ]; then # Debug for the first node only
        echo "[DEBUG] Content of /shared/node-keys/$node_name/keys/sidechain.json:"
        cat "/shared/node-keys/$node_name/keys/sidechain.json"
        echo "[DEBUG] End of sidechain.json content for $node_name."
    fi

    ./partner-chains-node key generate \
        --scheme sr25519 \
        --output-type json \
        > /shared/node-keys/$node_name/keys/aura.json
    
    if [ "$i" -eq 1 ]; then # Debug for the first node only
        echo "[DEBUG] Content of /shared/node-keys/$node_name/keys/aura.json:"
        cat "/shared/node-keys/$node_name/keys/aura.json"
        echo "[DEBUG] End of aura.json content for $node_name."
    fi
    
    ./partner-chains-node key generate \
        --scheme ed25519 \
        --output-type json \
        > /shared/node-keys/$node_name/keys/grandpa.json
    
    # Create keystore files from secret phrases
    sidechain_phrase=$(jq -r '.secretPhrase' "/shared/node-keys/$node_name/keys/sidechain.json")
    sidechain_pubkey=$(jq -r '.publicKey' "/shared/node-keys/$node_name/keys/sidechain.json" | sed 's/0x//')
    echo "\"$sidechain_phrase\"" > "/shared/node-keys/$node_name/keystore/63726368${sidechain_pubkey}"
    
    aura_phrase=$(jq -r '.secretPhrase' "/shared/node-keys/$node_name/keys/aura.json")
    aura_pubkey=$(jq -r '.publicKey' "/shared/node-keys/$node_name/keys/aura.json" | sed 's/0x//')
    echo "\"$aura_phrase\"" > "/shared/node-keys/$node_name/keystore/61757261${aura_pubkey}"

    grandpa_phrase=$(jq -r '.secretPhrase' "/shared/node-keys/$node_name/keys/grandpa.json")
    grandpa_pubkey=$(jq -r '.publicKey' "/shared/node-keys/$node_name/keys/grandpa.json" | sed 's/0x//')
    echo "\"$grandpa_phrase\"" > "/shared/node-keys/$node_name/keystore/6772616e${grandpa_pubkey}"
    
    # Allow node process to write to its directory (for keystore etc.)
    chmod -R 777 "/shared/node-keys/$node_name"
    
    if [ "$i" -eq 1 ]; then # Debug for the first node only
        echo "[DEBUG] Content of /shared/node-keys/$node_name/keys/grandpa.json:"
        cat "/shared/node-keys/$node_name/keys/grandpa.json"
        echo "[DEBUG] End of grandpa.json content for $node_name."
    fi
    
    # Extract public keys
    sidechain_vkey=$(jq -r '.publicKey' /shared/node-keys/$node_name/keys/sidechain.json)
    aura_vkey=$(jq -r '.publicKey' /shared/node-keys/$node_name/keys/aura.json)
    grandpa_vkey=$(jq -r '.publicKey' /shared/node-keys/$node_name/keys/grandpa.json)
    
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
for ((i=1; i<=NUM_REGISTERED_NODES_TO_PROCESS; i++)); do
    node_name="registered-$i"
    echo "Processing $node_name..."
    
    # Create directory for node keys
    mkdir -p /shared/node-keys/$node_name/keys
    mkdir -p /shared/node-keys/$node_name/keystore
    
    # Generate keys for each node
    ./partner-chains-node key generate \
        --scheme ecdsa \
        --output-type json \
        > /shared/node-keys/$node_name/keys/sidechain.json
    
    ./partner-chains-node key generate \
        --scheme sr25519 \
        --output-type json \
        > /shared/node-keys/$node_name/keys/aura.json
    
    ./partner-chains-node key generate \
        --scheme ed25519 \
        --output-type json \
        > /shared/node-keys/$node_name/keys/grandpa.json
    
    # Create keystore files from secret phrases
    sidechain_phrase=$(jq -r '.secretPhrase' "/shared/node-keys/$node_name/keys/sidechain.json")
    sidechain_pubkey=$(jq -r '.publicKey' "/shared/node-keys/$node_name/keys/sidechain.json" | sed 's/0x//')
    echo "\"$sidechain_phrase\"" > "/shared/node-keys/$node_name/keystore/63726368${sidechain_pubkey}"

    aura_phrase=$(jq -r '.secretPhrase' "/shared/node-keys/$node_name/keys/aura.json")
    aura_pubkey=$(jq -r '.publicKey' "/shared/node-keys/$node_name/keys/aura.json" | sed 's/0x//')
    echo "\"$aura_phrase\"" > "/shared/node-keys/$node_name/keystore/61757261${aura_pubkey}"

    grandpa_phrase=$(jq -r '.secretPhrase' "/shared/node-keys/$node_name/keys/grandpa.json")
    grandpa_pubkey=$(jq -r '.publicKey' "/shared/node-keys/$node_name/keys/grandpa.json" | sed 's/0x//')
    echo "\"$grandpa_phrase\"" > "/shared/node-keys/$node_name/keystore/6772616e${grandpa_pubkey}"
    
    # Allow node process to write to its directory (for keystore etc.)
    chmod -R 777 "/shared/node-keys/$node_name"

    # Extract keys and generate signatures
    sidechain_signing_key=$(jq -r '.secretSeed' /shared/node-keys/$node_name/keys/sidechain.json)
    
    # Define and read the specific registration UTXO for this node
    NODE_REGISTRATION_UTXO_FILE="/shared/registered-${i}.utxo"
    if [ ! -f "$NODE_REGISTRATION_UTXO_FILE" ]; then
        echo "Error: Registration UTXO file $NODE_REGISTRATION_UTXO_FILE not found for $node_name!"
        exit 1
    fi
    echo "[DEBUG] Content of $NODE_REGISTRATION_UTXO_FILE for $node_name:"
    cat "$NODE_REGISTRATION_UTXO_FILE"
    echo "[DEBUG] End content of $NODE_REGISTRATION_UTXO_FILE for $node_name."

    NODE_REGISTRATION_UTXO=$(cat "$NODE_REGISTRATION_UTXO_FILE")
    if [ -z "$NODE_REGISTRATION_UTXO" ]; then
        echo "Error: Registration UTXO file $NODE_REGISTRATION_UTXO_FILE is empty for $node_name!"
        exit 1
    fi
    echo "Using registration UTXO $NODE_REGISTRATION_UTXO for $node_name"
    echo "[DEBUG] Value of NODE_REGISTRATION_UTXO variable: [$NODE_REGISTRATION_UTXO]"

    # Define and read the specific mainchain cold signing key for this node
    NODE_MAINCHAIN_SKEY_FILE="/shared/node-keys/registered-${i}/keys/stake.skey"
    if [ ! -f "$NODE_MAINCHAIN_SKEY_FILE" ]; then
        echo "Error: Mainchain signing key file $NODE_MAINCHAIN_SKEY_FILE not found for $node_name!"
        exit 1 # This is a critical failure
    fi
    # Extract the raw key (cborHex, stripping the '5820' prefix)
    MAINCHAIN_SIGNING_KEY_RAW=$(jq -r '.cborHex | .[4:]' "$NODE_MAINCHAIN_SKEY_FILE")
    if [ -z "$MAINCHAIN_SIGNING_KEY_RAW" ] || [ "${#MAINCHAIN_SIGNING_KEY_RAW}" -ne 64 ]; then # 32 bytes = 64 hex chars
        echo "Error: Failed to extract raw mainchain signing key or key is invalid for $node_name from $NODE_MAINCHAIN_SKEY_FILE!"
        jq '.' "$NODE_MAINCHAIN_SKEY_FILE" # Print file content for debugging
        exit 1 # This is a critical failure
    fi
    echo "Using mainchain signing key from $NODE_MAINCHAIN_SKEY_FILE for $node_name"
    echo "[DEBUG] About to call registration-signatures for $node_name with UTXO: [$NODE_REGISTRATION_UTXO]"

    # Process registration signatures
    registration_output=$(./partner-chains-node registration-signatures \
        --genesis-utxo $GENESIS_UTXO \
        --mainchain-signing-key $MAINCHAIN_SIGNING_KEY_RAW \
        --sidechain-signing-key $sidechain_signing_key \
        --registration-utxo $NODE_REGISTRATION_UTXO)
    
    # Extract signatures and keys
    spo_public_key=$(echo "$registration_output" | jq -r ".spo_public_key")
    spo_signature=$(echo "$registration_output" | jq -r ".spo_signature")
    sidechain_public_key=$(echo "$registration_output" | jq -r ".sidechain_public_key")
    sidechain_signature=$(echo "$registration_output" | jq -r ".sidechain_signature")
    aura_vkey=$(jq -r '.publicKey' /shared/node-keys/$node_name/keys/aura.json)
    grandpa_vkey=$(jq -r '.publicKey' /shared/node-keys/$node_name/keys/grandpa.json)
    
    NODE_PAYMENT_SKEY_FILE="/shared/node-keys/registered-${i}/keys/payment.skey"
    if [ ! -f "$NODE_PAYMENT_SKEY_FILE" ]; then
        echo "Error: Payment signing key file $NODE_PAYMENT_SKEY_FILE not found for $node_name!"
        exit 1 # This is a critical failure
    fi
    echo "Using payment key $NODE_PAYMENT_SKEY_FILE for $node_name registration transaction."
    echo "[DEBUG] About to call smart-contracts register for $node_name with UTXO: [$NODE_REGISTRATION_UTXO]"

    # Register the node
    ./partner-chains-node smart-contracts register \
        --ogmios-url http://ogmios:$OGMIOS_PORT \
        --genesis-utxo $GENESIS_UTXO \
        --spo-public-key $spo_public_key \
        --spo-signature $spo_signature \
        --sidechain-public-keys $sidechain_public_key:$aura_vkey:$grandpa_vkey \
        --sidechain-signature $sidechain_signature \
        --registration-utxo $NODE_REGISTRATION_UTXO \
        --payment-key-file $NODE_PAYMENT_SKEY_FILE

    if [ $? -ne 0 ]; then
        echo "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
        echo "ERROR: smart-contracts register failed for $node_name"
        echo "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
    fi
done

echo "Generating chain-spec.json file for Partnerchain Nodes..."
./partner-chains-node build-spec --disable-default-bootnode > chain-spec.json

echo "Configuring Initial Validators with SS58 Address ID..."
echo "[" > initial_validators.json
for ((i=1; i<=NUM_PERMISSIONED_NODES_TO_PROCESS; i++)); do
    node_name="permissioned-$i"
    validator_id=$(jq -r '.ss58Address' "/shared/node-keys/$node_name/keys/sidechain.json")
    aura_ss58=$(jq -r '.ss58Address' "/shared/node-keys/$node_name/keys/aura.json")
    grandpa_ss58=$(jq -r '.ss58Address' "/shared/node-keys/$node_name/keys/grandpa.json")
    
    if [ $i -gt 1 ]; then
        echo "," >> initial_validators.json
    fi
    
    cat <<EOF >> initial_validators.json
    [
        "$validator_id",
        {
            "aura": "$aura_ss58",
            "grandpa": "$grandpa_ss58"
        }
    ]
EOF
done
echo "]" >> initial_validators.json

# Update chain-spec.json with initial validators
jq --slurpfile validators initial_validators.json '.genesis.runtimeGenesis.config.session.initialValidators = $validators[0]' chain-spec.json > chain-spec.json.tmp
mv chain-spec.json.tmp chain-spec.json

echo "Configuring Initial Authorities with SS58 Public Key ID..."
echo "[" > initial_authorities.json
for ((i=1; i<=NUM_PERMISSIONED_NODES_TO_PROCESS; i++)); do
    node_name="permissioned-$i"
    validator_id=$(jq -r '.ss58PublicKey' "/shared/node-keys/$node_name/keys/sidechain.json")
    aura_ss58=$(jq -r '.ss58Address' "/shared/node-keys/$node_name/keys/aura.json")
    grandpa_ss58=$(jq -r '.ss58Address' "/shared/node-keys/$node_name/keys/grandpa.json")
    
    if [ $i -gt 1 ]; then
        echo "," >> initial_authorities.json
    fi
    
    cat <<EOF >> initial_authorities.json
    {
        "Permissioned": {
            "id": "$validator_id",
            "keys": {
                "aura": "$aura_ss58",
                "grandpa": "$grandpa_ss58"
            }
        }
    }
EOF
done
echo "]" >> initial_authorities.json

jq --slurpfile authorities initial_authorities.json '.genesis.runtimeGenesis.config.sessionCommitteeManagement.initialAuthorities = $authorities[0]' chain-spec.json > chain-spec.json.tmp
mv chain-spec.json.tmp chain-spec.json
rm initial_authorities.json # Clean up temporary file

echo "Setting Governed Map scripts..."
export GOVERNED_MAP_VALIDATOR_ADDRESS_HEX="0x$(echo -n $GOVERNED_MAP_VALIDATOR_ADDRESS | xxd -p -c 128)"
jq --arg address $GOVERNED_MAP_VALIDATOR_ADDRESS_HEX --arg policy_id $GOVERNED_MAP_POLICY_ID '.genesis.runtimeGenesis.config.governedMap.mainChainScripts = {
  "validator_address": $address,
  "asset_policy_id": $policy_id
}' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Configuring Initial Balances..."
# Fund each of the 10 permissioned nodes (using their ECDSA sidechain public key)
initial_balance_amount="1000000000000000"
echo "[" > initial_balances.json
for ((i=1; i<=NUM_PERMISSIONED_NODES_TO_PROCESS; i++)); do
    node_name="permissioned-$i"
    account_id=$(jq -r '.ss58Address' "/shared/node-keys/$node_name/keys/sidechain.json") # MODIFIED PATH

    if [ $i -gt 1 ]; then
        echo "," >> initial_balances.json
    fi
    cat <<EOF >> initial_balances.json
    [
        "$account_id",
        $initial_balance_amount
    ]
EOF
done
echo "]" >> initial_balances.json
jq --slurpfile balances initial_balances.json '.genesis.runtimeGenesis.config.balances.balances = $balances[0]' chain-spec.json > chain-spec.json.tmp
mv chain-spec.json.tmp chain-spec.json
rm initial_balances.json # Clean up temporary file

echo "Configuring Sudo Key..."
# Use the Aura ss58Address of the first permissioned node as the sudo key
sudo_account_key=$(jq -r '.ss58Address' "/shared/node-keys/permissioned-1/keys/aura.json") # MODIFIED PATH
jq --arg sudo_key "$sudo_account_key" '.genesis.runtimeGenesis.config.sudo = { "key": $sudo_key }' chain-spec.json > chain-spec.json.tmp
mv chain-spec.json.tmp chain-spec.json

echo "Configuring Epoch Length..."
jq '.genesis.runtimeGenesis.config.sidechain.slotsPerEpoch = 5' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

cp chain-spec.json /shared/chain-spec.json
echo "chain-spec.json generation complete."

echo "Setting final permissions for all shared keys and keystores..."
chmod -R 777 /shared/node-keys

touch /shared/chain-spec.ready

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

echo "Signaling to nodes that setup is complete."
touch /shared/partner-chains-setup.ready

tail -f /dev/null
