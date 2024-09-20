#!/bin/bash

# Initialize flags
PC_NODE_READY=0
PC_CLI_READY=0
PC_CONTRACTS_CLI_READY=0

if [ "$ARTIFACT_OVERRIDE" == "yes" ]; then
  echo "Artifact override is enabled. Checking for local artifacts..."

  # Check and set flags for existing artifacts, and copy if found
  if [ -f "/overrides/pc-contracts-cli" ] && [ -d "/overrides/node_modules" ]; then
    echo "pc-contracts-cli and node_modules found in /overrides/. Using local artifacts."
    cp /overrides/pc-contracts-cli ./pc-contracts-cli
    cp -r /overrides/node_modules ./node_modules
    echo "pc-contracts-cli and node_modules copied."
    PC_CONTRACTS_CLI_READY=1
  fi

  if [ -f "/overrides/partner-chains-node" ]; then
    echo "partner-chains-node found in /overrides/. Using local artifact."
    cp /overrides/partner-chains-node ./partner-chains-node
    echo "partner-chains-node copied."
    PC_NODE_READY=1
  fi

  if [ -f "/overrides/partner-chains-cli" ]; then
    echo "partner-chains-cli found in /overrides/. Using local artifact."
    cp /overrides/partner-chains-cli ./partner-chains-cli
    echo "partner-chains-cli copied."
    PC_CLI_READY=1
  fi

else
  echo "Artifact override is not enabled. Defaulting to downloading all artifacts..."
fi

# Check which artifacts need to be downloaded
if [ "$PC_CONTRACTS_CLI_READY" -eq 0 ]; then
  echo "Downloading pc-contracts-cli and node_modules..."
  wget -q -O ./pc-contracts-cli.zip "$PC_CONTRACTS_CLI_ZIP_URL"
  unzip -o ./pc-contracts-cli.zip > /dev/null
  mv ./sidechain-cli ./pc-contracts-cli
fi

if [ "$PC_NODE_READY" -eq 0 ]; then
  echo "Downloading partner-chains-node..."
  wget -q -O ./partner-chains-node "$PARTNER_CHAINS_NODE_URL"
fi

if [ "$PC_CLI_READY" -eq 0 ]; then
  echo "Downloading partner-chains-cli..."
  wget -q -O ./partner-chains-cli "$PARTNER_CHAINS_CLI_URL"
fi

# Set executable permissions
chmod +x ./partner-chains-node
chmod +x ./partner-chains-cli
chmod +x ./pc-contracts-cli

# Install jq
apt -qq update &> /dev/null
apt -qq -y install jq ncat &> /dev/null

echo "Dependencies downloaded and binaries made executable."

echo "Waiting for the Cardano network to sync and for Kupo and Ogmios to start..."

while true; do
    if nc -z kupo $KUPO_PORT && nc -z ogmios $OGMIOS_PORT; then
        break
    else
        sleep 10
    fi
done

echo "Beginning configuration..."

chmod 644 /shared/shelley/genesis-utxo.skey

echo "Generating addresses.json file..."

./pc-contracts-cli addresses \
    --network testnet \
    --kupo-host kupo --kupo-port $KUPO_PORT \
    --ogmios-host ogmios --ogmios-port $OGMIOS_PORT \
    --payment-signing-key-file /keys/funded_address.skey \
    --genesis-committee-hash-utxo $GENESIS_COMMITTEE_UTXO \
    --sidechain-id $CHAIN_ID --threshold-numerator $THRESHOLD_NUMERATOR --threshold-denominator $THRESHOLD_DENOMINATOR \
    --governance-authority $GOVERNANCE_AUTHORITY \
    --version 1 \
    --atms-kind plain-ecdsa-secp256k1 \
> addresses.json

export COMMITTEE_CANDIDATE_ADDRESS=$(jq -r '.addresses.CommitteeCandidateValidator' addresses.json)
echo "Committee candidate address: $COMMITTEE_CANDIDATE_ADDRESS"
echo COMMITTEE_CANDIDATE_ADDRESS=$COMMITTEE_CANDIDATE_ADDRESS > /shared/COMMITTEE_CANDIDATE_ADDRESS

export D_PARAMETER_POLICY_ID=$(jq -r '.mintingPolicies.DParameterPolicy' addresses.json)
echo "D parameter policy ID: $D_PARAMETER_POLICY_ID"
echo D_PARAMETER_POLICY_ID=$D_PARAMETER_POLICY_ID > /shared/D_PARAMETER_POLICY_ID

export PERMISSIONED_CANDIDATES_POLICY_ID=$(jq -r '.mintingPolicies.PermissionedCandidatesPolicy' addresses.json)
echo "Permissioned candidates policy ID: $PERMISSIONED_CANDIDATES_POLICY_ID"
echo PERMISSIONED_CANDIDATES_POLICY_ID=$PERMISSIONED_CANDIDATES_POLICY_ID > /shared/PERMISSIONED_CANDIDATES_POLICY_ID

echo "Inserting D parameter..."

./pc-contracts-cli insert-d-parameter \
    --network testnet \
    --kupo-host kupo --kupo-port $KUPO_PORT \
    --ogmios-host ogmios --ogmios-port $OGMIOS_PORT \
    --genesis-committee-hash-utxo $GENESIS_COMMITTEE_UTXO \
    --sidechain-id $CHAIN_ID --threshold-numerator $THRESHOLD_NUMERATOR --threshold-denominator $THRESHOLD_DENOMINATOR \
    --governance-authority $GOVERNANCE_AUTHORITY \
    --atms-kind plain-ecdsa-secp256k1 \
    --d-parameter-permissioned-candidates-count 3 \
    --d-parameter-registered-candidates-count 2 \
    --payment-signing-key-file /keys/funded_address.skey

echo "Successfully inserted D-parameter (P = 3, R = 2)!"

# sidechain.vkey:aura.vkey:grandpa.vkey
echo "Inserting permissioned candidates for Alice and Bob..."

alice_sidechain_vkey=$(cat /partner-chains-nodes/partner-chains-node-1/keys/sidechain.vkey)
alice_aura_vkey=$(cat /partner-chains-nodes/partner-chains-node-1/keys/aura.vkey)
alice_grandpa_vkey=$(cat /partner-chains-nodes/partner-chains-node-1/keys/grandpa.vkey)

bob_sidechain_vkey=$(cat /partner-chains-nodes/partner-chains-node-2/keys/sidechain.vkey)
bob_aura_vkey=$(cat /partner-chains-nodes/partner-chains-node-2/keys/aura.vkey)
bob_grandpa_vkey=$(cat /partner-chains-nodes/partner-chains-node-2/keys/grandpa.vkey)

./pc-contracts-cli update-permissioned-candidates \
    --network testnet \
    --kupo-host kupo --kupo-port $KUPO_PORT \
    --ogmios-host ogmios --ogmios-port $OGMIOS_PORT \
    --add-candidate $alice_sidechain_vkey:$alice_aura_vkey:$alice_grandpa_vkey \
    --add-candidate $bob_sidechain_vkey:$bob_aura_vkey:$bob_grandpa_vkey \
    --genesis-committee-hash-utxo $GENESIS_COMMITTEE_UTXO \
    --governance-authority $GOVERNANCE_AUTHORITY \
    --threshold-numerator $THRESHOLD_NUMERATOR \
    --threshold-denominator $THRESHOLD_DENOMINATOR \
    --sidechain-id 0 \
    --atms-kind plain-ecdsa-secp256k1 \
    --payment-signing-key-file /keys/funded_address.skey

if [ $? -eq 0 ]; then
   echo "Permissioned candidates Alice and Bob inserted successfully!"
else
    echo "Permission candidates Alice and Bob failed to be added..."
fi

echo "Inserting registered candidate Dave..."

# Prepare Dave registration values
dave_utxo=$(cat /shared/dave.utxo)
dave_mainchain_signing_key=$(jq -r '.cborHex | .[4:]' /partner-chains-nodes/partner-chains-node-4/keys/cold.skey)
dave_sidechain_signing_key=$(cat /partner-chains-nodes/partner-chains-node-4/keys/sidechain.skey)

# Process registration signatures for Dave
dave_output=$(./partner-chains-node registration-signatures \
    --chain-id 0 \
    --genesis-committee-utxo $GENESIS_COMMITTEE_UTXO \
    --governance-authority $GOVERNANCE_AUTHORITY \
    --mainchain-signing-key $dave_mainchain_signing_key \
    --sidechain-signing-key $dave_sidechain_signing_key \
    --registration-utxo $dave_utxo \
    --threshold-numerator 2 --threshold-denominator 3)

# Extract signatures and keys from Dave output
dave_spo_public_key=$(echo "$dave_output" | jq -r ".spo_public_key")
dave_spo_signature=$(echo "$dave_output" | jq -r ".spo_signature")
dave_sidechain_public_key=$(echo "$dave_output" | jq -r ".sidechain_public_key")
dave_sidechain_signature=$(echo "$dave_output" | jq -r ".sidechain_signature")
dave_aura_vkey=$(cat /partner-chains-nodes/partner-chains-node-4/keys/aura.vkey)
dave_grandpa_vkey=$(cat /partner-chains-nodes/partner-chains-node-4/keys/grandpa.vkey)

# Register Dave
./pc-contracts-cli register \
    --network testnet \
    --kupo-host kupo --kupo-port $KUPO_PORT \
    --ogmios-host ogmios --ogmios-port $OGMIOS_PORT \
    --sidechain-id 0 \
    --genesis-committee-hash-utxo $GENESIS_COMMITTEE_UTXO \
    --governance-authority $GOVERNANCE_AUTHORITY \
    --atms-kind plain-ecdsa-secp256k1 \
    --threshold-numerator 2 \
    --threshold-denominator 3 \
    --spo-public-key $dave_spo_public_key \
    --spo-signature $dave_spo_signature \
    --sidechain-public-keys $dave_sidechain_public_key:$dave_aura_vkey:$dave_grandpa_vkey \
    --sidechain-signature $dave_sidechain_signature \
    --ada-based-staking \
    --registration-utxo $dave_utxo \
    --payment-signing-key-file /partner-chains-nodes/partner-chains-node-4/keys/payment.skey

if [ $? -eq 0 ]; then
    echo "Registered candidate Dave inserted successfully!"
else
    echo "Registration for Dave failed."
fi

echo "Generating chain-spec.json file for Parterchain Nodes..."
./partner-chains-node build-spec --disable-default-bootnode > chain-spec.json

echo "Configuring Initial Validators..."
jq '.genesis.runtimeGenesis.config.session.initialValidators = [
     [
         "5C7C2Z5sWbytvHpuLTvzKunnnRwQxft1jiqrLD5rhucQ5S9X",
         {
             "aura": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
             "grandpa": "5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu"
         }
     ],
     [
         "5DVskgSC9ncWQpxFMeUn45NU43RUq93ByEge6ApbnLk6BR9N",
         {
             "aura": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
             "grandpa": "5GoNkf6WdbxCFnPdAnYYQyCjAKPJgLNxXwPjwTh6DGg6gN3E"
         }
     ],
     [
         "5EP2cMaCxLzhfD3aFAqqgu3kfXH7GcwweEv6JXZRP6ysRHkQ",
         {
             "aura": "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
             "grandpa": "5DbKjhNLpqX3zqZdNBc9BGb4fHU1cRBaDhJUskrvkwfraDi6"
         }
     ]
 ]' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Configuring Initial Authorities..."
jq '.genesis.runtimeGenesis.config.sessionCommitteeManagement.initialAuthorities = [
     [
         "KW39r9CJjAVzmkf9zQ4YDb2hqfAVGdRqn53eRqyruqpxAP5YL",
         {
             "aura": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
             "grandpa": "5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu"
         }
     ],
     [
         "KWByAN7WfZABWS5AoWqxriRmF5f2jnDqy3rB5pfHLGkY93ibN",
         {
             "aura": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
             "grandpa": "5GoNkf6WdbxCFnPdAnYYQyCjAKPJgLNxXwPjwTh6DGg6gN3E"
         }
     ],
     [
         "KWBpGtyJLBkJERdZT1a1uu19c2uPpZm9nFd8SGtCfRUAT3Y4w",
         {
             "aura": "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
             "grandpa": "5DbKjhNLpqX3zqZdNBc9BGb4fHU1cRBaDhJUskrvkwfraDi6"
         }
     ]
 ]' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Configuring Balances..."
jq '.genesis.runtimeGenesis.config.balances.balances = [
    ["5C7C2Z5sWbytvHpuLTvzKunnnRwQxft1jiqrLD5rhucQ5S9X", 1000000000000000]
]' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Configuring Epoch Length..."
jq '.genesis.runtimeGenesis.config.sidechain.slotsPerEpoch = 5' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Copying chain-spec.json file to /shared/chain-spec.json..."
cp chain-spec.json /shared/chain-spec.json 
echo "chain-spec.json generation complete."

echo "Partnerchain configuration is complete, and will be able to start after two mainchain epochs."
touch /shared/pc-contracts-cli.ready

echo -e "\n===== Partnerchain Configuration Complete =====\n"
echo -e "Container will now idle, but will remain available for accessing the pc-contracts-cli utility as follows:\n"
echo "docker exec pc-contracts-cli /pc-contracts-cli/pc-contracts-cli --help"

epoch_length=$(cat /shared/mc-epoch-length)
slot_length=$(cat /shared/mc-slot-length)
sleep_time=$((2 * epoch_length * slot_length))
sleep $sleep_time
touch /shared/2-epochs.ready

tail -f /dev/null
'
