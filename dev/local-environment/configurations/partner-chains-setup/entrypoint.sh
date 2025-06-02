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
    --permissioned-candidates-count 3 \
    --registered-candidates-count 2 \
    --payment-key-file /keys/funded_address.skey

if [ $? -eq 0 ]; then
    echo "Successfully inserted D-parameter (P = 3, R = 2)!"
else
    echo "Couldn't insert D-parameter..."
    exit 1
fi

# sidechain.vkey:aura.vkey:grandpa.vkey
echo "Inserting permissioned candidates for 'node-1' and 'node-2'..."

node1_sidechain_vkey=$(cat /partner-chains-nodes/partner-chains-node-1/keys/sidechain.vkey)
node1_aura_vkey=$(cat /partner-chains-nodes/partner-chains-node-1/keys/aura.vkey)
node1_grandpa_vkey=$(cat /partner-chains-nodes/partner-chains-node-1/keys/grandpa.vkey)

node2_sidechain_vkey=$(cat /partner-chains-nodes/partner-chains-node-2/keys/sidechain.vkey)
node2_aura_vkey=$(cat /partner-chains-nodes/partner-chains-node-2/keys/aura.vkey)
node2_grandpa_vkey=$(cat /partner-chains-nodes/partner-chains-node-2/keys/grandpa.vkey)

cat <<EOF > permissioned_candidates.csv
$node1_sidechain_vkey:$node1_aura_vkey:$node1_grandpa_vkey
$node2_sidechain_vkey:$node2_aura_vkey:$node2_grandpa_vkey
EOF

./partner-chains-node smart-contracts upsert-permissioned-candidates \
    --ogmios-url http://ogmios:$OGMIOS_PORT \
    --genesis-utxo $GENESIS_UTXO \
    --permissioned-candidates-file permissioned_candidates.csv \
    --payment-key-file /keys/funded_address.skey

if [ $? -eq 0 ]; then
    echo "Permissioned candidates 'node-1' and 'node-2' inserted successfully!"
else
    echo "Permission candidates 'node-1' and 'node-2' failed to be added..."
    exit 1
fi

echo "Inserting registered candidate 'node-4'..."

# Prepare 'node-4' registration values
node4_utxo=$(cat /shared/node4.utxo)
node4_mainchain_signing_key=$(jq -r '.cborHex | .[4:]' /partner-chains-nodes/partner-chains-node-4/keys/cold.skey)
node4_sidechain_signing_key=$(cat /partner-chains-nodes/partner-chains-node-4/keys/sidechain.skey)

# Process registration signatures for node-4
node4_output=$(./partner-chains-node registration-signatures \
    --genesis-utxo $GENESIS_UTXO \
    --mainchain-signing-key $node4_mainchain_signing_key \
    --sidechain-signing-key $node4_sidechain_signing_key \
    --registration-utxo $node4_utxo)

# Extract signatures and keys from 'node-4' output
node4_spo_public_key=$(echo "$node4_output" | jq -r ".spo_public_key")
node4_spo_signature=$(echo "$node4_output" | jq -r ".spo_signature")
node4_sidechain_public_key=$(echo "$node4_output" | jq -r ".sidechain_public_key")
node4_sidechain_signature=$(echo "$node4_output" | jq -r ".sidechain_signature")
node4_aura_vkey=$(cat /partner-chains-nodes/partner-chains-node-4/keys/aura.vkey)
node4_grandpa_vkey=$(cat /partner-chains-nodes/partner-chains-node-4/keys/grandpa.vkey)

# Register 'node-4'
./partner-chains-node smart-contracts register \
    --ogmios-url http://ogmios:$OGMIOS_PORT \
    --genesis-utxo $GENESIS_UTXO \
    --spo-public-key $node4_spo_public_key \
    --spo-signature $node4_spo_signature \
    --sidechain-public-keys $node4_sidechain_public_key:$node4_aura_vkey:$node4_grandpa_vkey \
    --sidechain-signature $node4_sidechain_signature \
    --registration-utxo $node4_utxo \
    --payment-key-file /partner-chains-nodes/partner-chains-node-4/keys/payment.skey

if [ $? -eq 0 ]; then
    echo "Registered candidate 'node-4' inserted successfully!"
else
    echo "Registration for 'node-4' failed."
    exit 1
fi

echo "Inserting registered candidate 'node-5'..."

# Prepare 'node-5' registration values
node5_utxo=$(cat /shared/node5.utxo)
node5_mainchain_signing_key=$(jq -r '.cborHex | .[4:]' /partner-chains-nodes/partner-chains-node-5/keys/cold.skey)
node5_sidechain_signing_key=$(cat /partner-chains-nodes/partner-chains-node-5/keys/sidechain.skey)

# Process registration signatures for node-5
node5_output=$(./partner-chains-node registration-signatures \
    --genesis-utxo $GENESIS_UTXO \
    --mainchain-signing-key $node5_mainchain_signing_key \
    --sidechain-signing-key $node5_sidechain_signing_key \
    --registration-utxo $node5_utxo)

# Extract signatures and keys from node-5 output
node5_spo_public_key=$(echo "$node5_output" | jq -r ".spo_public_key")
node5_spo_signature=$(echo "$node5_output" | jq -r ".spo_signature")
node5_sidechain_public_key=$(echo "$node5_output" | jq -r ".sidechain_public_key")
node5_sidechain_signature=$(echo "$node5_output" | jq -r ".sidechain_signature")
node5_aura_vkey=$(cat /partner-chains-nodes/partner-chains-node-5/keys/aura.vkey)
node5_grandpa_vkey=$(cat /partner-chains-nodes/partner-chains-node-5/keys/grandpa.vkey)

# Register 'node-5'
./partner-chains-node smart-contracts register \
    --ogmios-url http://ogmios:$OGMIOS_PORT \
    --genesis-utxo $GENESIS_UTXO \
    --spo-public-key $node5_spo_public_key \
    --spo-signature $node5_spo_signature \
    --sidechain-public-keys $node5_sidechain_public_key:$node5_aura_vkey:$node5_grandpa_vkey \
    --sidechain-signature $node5_sidechain_signature \
    --registration-utxo $node5_utxo \
    --payment-key-file /partner-chains-nodes/partner-chains-node-5/keys/payment.skey

if [ $? -eq 0 ]; then
    echo "Registered candidate 'node-5' inserted successfully!"
else
    echo "Registration for 'node-5' failed."
    exit 1
fi

echo "Generating chain-spec.json file for Partner chain Nodes..."
./partner-chains-node build-spec --disable-default-bootnode > chain-spec.json

echo "Configuring Initial Validators..."
jq '.genesis.runtimeGenesis.config.session.initialValidators = [
     [
         "5FnXTMg8UnfeGsMaGg24o3NY21VRFRDRdgxuLGmXuYLeZmin",
         {
             "aura": "5Cyx94iyji8namhRxvs4mAbURtPsvwjWCb68ZihNzfRysGLZ",
             "grandpa": "5Cc5eQhbPw4CjwZpWqZkWWumMiuZywfWRK2Rh9guXUJ3U89s"
         }
     ],
     [
         "5FJMH4MeZgd4fpiiAVLnr4uRop2EDFgzAFcvLmcduQ2cofCi",
         {
             "aura": "5E4op92Z2Di1GoVS9KqnoGVKQXG2R9x1vdh3RW892YLFsLrc",
             "grandpa": "5Ha53RXoJjXtcTThFA5XNW7H6f5L39HnTuVSXimxAyhoYLeL"
         }
     ]
 ]' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Configuring Initial Authorities..."
jq '.genesis.runtimeGenesis.config.sessionCommitteeManagement.initialAuthorities = [
  {
    "Permissioned": {
      "id": "KW4wALva83fvah66ufXSxg6r84tTpJmDXna8A1PCYdbZdVL95",
      "keys": {
        "aura": "5Cyx94iyji8namhRxvs4mAbURtPsvwjWCb68ZihNzfRysGLZ",
        "grandpa": "5Cc5eQhbPw4CjwZpWqZkWWumMiuZywfWRK2Rh9guXUJ3U89s"
      }
    }
  },
  {
    "Permissioned": {
      "id": "KW92jBDRydnbyojCVF3USNFgEsrEvDGV3gvdgDvpfnbXvC13q",
      "keys": {
        "aura": "5E4op92Z2Di1GoVS9KqnoGVKQXG2R9x1vdh3RW892YLFsLrc",
        "grandpa": "5Ha53RXoJjXtcTThFA5XNW7H6f5L39HnTuVSXimxAyhoYLeL"
      }
    }
  }
]' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Setting Governed Map scripts..."
export GOVERNED_MAP_VALIDATOR_ADDRESS_HEX="0x$(echo -n $GOVERNED_MAP_VALIDATOR_ADDRESS | xxd -p -c 128)"
jq --arg address $GOVERNED_MAP_VALIDATOR_ADDRESS_HEX --arg policy_id $GOVERNED_MAP_POLICY_ID '.genesis.runtimeGenesis.config.governedMap.mainChainScripts = {
  "validator_address": $address,
  "asset_policy_id": $policy_id
}' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Set initial funds to node-1 (ECDSA), node-1 (sr25519), node-4 (ECDSA) and node-4 (sr25519)"
jq '.genesis.runtimeGenesis.config.balances.balances = [
    ["5FnXTMg8UnfeGsMaGg24o3NY21VRFRDRdgxuLGmXuYLeZmin", 1000000000000000],
    ["5Cyx94iyji8namhRxvs4mAbURtPsvwjWCb68ZihNzfRysGLZ", 1000000000000000],
    ["5GaTC1bjMYLxXo2DqnxxdCWLEdGZK86mWmSYtzkG6BKHzT2H", 1000000000000000],
    ["5HKLH5ErLMNHReWGFGtrDPRdNqdKP56ArQA6DFmgANzunK7A", 1000000000000000]
]' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Configuring node-1 (sr25519) as sudo..."
jq '.genesis.runtimeGenesis.config.sudo = {
    "key": "5Cyx94iyji8namhRxvs4mAbURtPsvwjWCb68ZihNzfRysGLZ"
}' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Configuring Epoch Length..."
jq '.genesis.runtimeGenesis.config.sidechain.slotsPerEpoch = 5' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Copying chain-spec.json file to /shared/chain-spec.json..."
cp chain-spec.json /shared/chain-spec.json
echo "chain-spec.json generation complete."

echo "Partner chain configuration is complete, and will be able to start after two mainchain epochs."
touch /shared/partner-chains-setup.ready

echo -e "\n===== Partner chain Configuration Complete =====\n"
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
