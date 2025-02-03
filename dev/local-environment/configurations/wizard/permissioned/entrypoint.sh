#!/bin/bash

echo "Installing dependencies..."

apt -qq update &> /dev/null
apt -qq -y install expect jq &> /dev/null

cp /usr/local/bin/partner-chains-node /partner-chains-node

echo "Beginning configuration..."


echo "Generating keys..."
expect <<EOF
spawn ./partner-chains-node wizards generate-keys
set timeout 60
expect "node base path (./data)"
send "\r"
expect "All done!"
expect eof
EOF

cp partner-chains-public-keys.json /shared/partner-chains-public-keys.json
touch /shared/partner-chains-wizard-2.ready


echo "Waiting for chain-spec.json and pc-chain-config.json to be ready..."
while true; do
    if [ -f "/shared/chain-spec.ready" ]; then
        break
    else
        sleep 1
    fi
done

cp /shared/chain-spec.json .
cp /shared/pc-chain-config.json .


echo "Configuring Node P2P port..."
jq '.node_p2p_port = 30334' pc-resources-config.json > tmp.json && mv tmp.json pc-resources-config.json

echo "Starting the node..."
expect <<EOF
spawn ./partner-chains-node wizards start-node
expect "DB-Sync Postgres connection string (postgresql://postgres-user:postgres-password@localhost:5432/cexplorer)"
send "postgresql://postgres:$POSTGRES_PASSWORD@postgres:$POSTGRES_PORT/cexplorer\r"
expect "Proceed? (Y/n)"
send "Y\r"
set timeout -1
expect eof
EOF
