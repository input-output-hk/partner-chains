#!/bin/bash

start_node() {
    echo "Starting the node..."
    export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)

    /usr/local/bin/partner-chains-node \
        --validator \
        --chain=/shared/chain-spec.json \
        --node-key-file=/data/chains/partner_chains_template/network/secret_ed25519 \
        --base-path=/data \
        --keystore-path=/data/chains/partner_chains_template/keystore \
        --unsafe-rpc-external \
        --rpc-port=9933 \
        --rpc-cors=all \
        --prometheus-port=9615 \
        --prometheus-external \
        --state-pruning=archive \
        --blocks-pruning=archive &
    wait
}

if [ -f "/shared/partner-chains-wizard-1.ready" ]; then
    echo "/shared/partner-chains-wizard-1.ready exists. Skipping configuration and starting the node..."
    start_node
    exit 0
fi


echo "Installing dependencies..."
apt -qq update &> /dev/null
apt -qq -y install expect curl jq ncat &> /dev/null
cp /usr/local/bin/partner-chains-node /partner-chains-node


echo "Beginning configuration..."
echo "Generating keys..."
expect <<EOF
spawn ./partner-chains-node wizards generate-keys
set timeout 60
expect "node base path (./data)"
send "\r"
expect eof
EOF


echo "Waiting for the Cardano network to sync and for Ogmios to start..."
while true; do
    if nc -z ogmios $OGMIOS_PORT; then
        break
    else
        sleep 1
    fi
done


echo "Preparing configuration..."
expect <<EOF
spawn ./partner-chains-node wizards prepare-configuration
set timeout 180
expect "node base path (./data)"
send "\r"
expect "Your bootnode should be accessible via:"
send "\r"
expect "Enter bootnode TCP port (3033)"
send "30333\r"
expect "Enter bootnode hostname (localhost)"
send "partner-chains-wizard-1\r"
expect "Ogmios protocol (http/https)"
send "\r"
expect "Ogmios hostname (localhost)"
send "ogmios\r"
expect "Ogmios port (1337)"
send "\r"
expect "path to the payment signing key file (payment.skey)"
send "/keys/funded_address.skey\r"
expect "Select an UTXO to use as the genesis UTXO"
send "\r"
expect "Space separated keys hashes of the initial Multisig Governance Authorities ()"
send "$GOVERNANCE_AUTHORITY\r"
expect "Initial Multisig Governance Threshold (0)"
send "1\r"
expect "Do you want to continue? (y/N)"
send "y\r"
expect "Do you want to configure a native token for you Partner Chain? (Y/n)"
send "n\r"
expect eof
EOF


echo "Waiting for permissioned candidate's keys to be generated..."
while true; do
    if [ -f "/shared/partner-chains-wizard-2-keys.ready" ]; then
        break
    else
        sleep 1
    fi
done


echo "Inserting permissioned candidates' keys into configuration..."
governance_authority_public_keys=$(cat partner-chains-public-keys.json)
permissioned_candidate_public_keys=$(cat /shared/partner-chains-public-keys.json)
jq '.initial_permissioned_candidates = [
    '"$governance_authority_public_keys"',
    '"$permissioned_candidate_public_keys"'
]' pc-chain-config.json > tmp.json && mv tmp.json pc-chain-config.json


echo "Creating chain spec..."
expect <<EOF
spawn ./partner-chains-node wizards create-chain-spec
expect "Do you want to continue? (Y/n)"
send "\r"
expect eof
EOF


echo "Set initial funds to Alice (ecdsa), ?, and Alice (sr25519)"
jq '.genesis.runtimeGenesis.config.balances.balances = [
    ["5C7C2Z5sWbytvHpuLTvzKunnnRwQxft1jiqrLD5rhucQ5S9X", 1000000000000000],
    ["5D9eDKbFt4JKaEndQvMmbJYnpX9ENUj8U9UUg1AxSa64FJxE", 1000000000000000],
    ["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", 1000000000000000]
]' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Configuring Alice as sudo..."
jq '.genesis.runtimeGenesis.config.sudo = {
    "key": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
}' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Configuring Epoch Length..."
jq '.genesis.runtimeGenesis.config.sidechain.slotsPerEpoch = 10' chain-spec.json > tmp.json && mv tmp.json chain-spec.json

echo "Copying chain-spec.json file to /shared/chain-spec.json..."
cp chain-spec.json /shared/chain-spec.json
echo "chain-spec.json generation complete."

echo "Copying pc-chain-config.json file to /shared/pc-chain-config.json..."
cp pc-chain-config.json /shared/pc-chain-config.json

touch /shared/chain-spec.ready


echo "Setting up main chain state..."
expect <<EOF
spawn ./partner-chains-node wizards setup-main-chain-state
set timeout 300
expect "DB-Sync Postgres connection string (postgresql://postgres-user:postgres-password@localhost:5432/cexplorer)"
send "postgresql://postgres:$POSTGRES_PASSWORD@postgres:$POSTGRES_PORT/cexplorer\r"
expect "Do you want to set/update the permissioned candidates on the main chain with values from configuration file? (y/N)"
send "y\r"
expect "Ogmios protocol (http/https)"
send "\r"
expect "Ogmios hostname (ogmios)"
send "\r"
expect "Ogmios port (1337)"
send "\r"
expect "path to the payment signing key file (/keys/funded_address.skey)"
send "\r"
expect "Do you want to set/update the D-parameter on the main chain? (y/N)"
send "y\r"
expect "Enter P, the number of permissioned candidates seats, as a non-negative integer. (0)"
send "2\r"
expect "Enter R, the number of registered candidates seats, as a non-negative integer. (0)"
send "1\r"
expect "path to the payment signing key file (/keys/funded_address.skey)"
send "\r"
expect "Done. Main chain state is set."
expect eof
EOF

touch /shared/partner-chains-wizard-1.ready
echo "Partner Chain configuration is complete, and will be able to start after two mainchain epochs."

start_node
