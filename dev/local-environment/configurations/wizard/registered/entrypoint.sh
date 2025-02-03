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
expect eof
EOF


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


echo "Registering candidate..."
register1_output=$(expect <<EOF
spawn ./partner-chains-node wizards register1
set timeout 60
expect "Ogmios protocol (http/https)"
send "\r"
expect "Ogmios hostname (localhost)"
send "ogmios\r"
expect "Ogmios port (1337)"
send "\r"
expect "path to the payment verification file (payment.vkey)"
send "keys/funded_address.vkey\r"
expect "Select UTXO to use for registration"
send "\r"
expect eof
catch wait result
exit [lindex \$result 3]
EOF
)

register2_command=$(echo "$register1_output" | sed -n '/\/partner-chains-node wizards register2 \\/,$p' | tr -d '\\\n')
echo "$register2_command"

echo "Running register2 command..."
register2_output=$(expect <<EOF
spawn $register2_command
set timeout 60
expect "Path to mainchain signing key file (cold.skey)"
send "keys/cold.skey\r"
expect "/partner-chains-node wizards register3"
expect eof
catch wait result
exit [lindex \$result 3]
EOF
)

register3_command=$(echo "$register2_output" | sed -n '/\/partner-chains-node wizards register3 \\/,$p' | tr -d '\\\n')
echo "$register3_command"

echo "Running register3 command..."
expect <<EOF
spawn $register3_command
set timeout 300
expect "Path to mainchain payment signing key file (payment.skey)"
send "keys/funded_address.skey\r"
expect "Ogmios protocol (http/https)"
send "\r"
expect "Ogmios hostname (ogmios)"
send "\r"
expect "Ogmios port (1337)"
send "\r"
expect "Show registration status? (Y/n)"
send "Y\r"
expect "DB-Sync Postgres connection string (postgresql://postgres-user:postgres-password@localhost:5432/cexplorer)"
send "postgresql://postgres:$POSTGRES_PASSWORD@postgres:$POSTGRES_PORT/cexplorer\r"
expect eof
EOF

echo "Configuring Node P2P port..."
jq '.node_p2p_port = 30335' pc-resources-config.json > tmp.json && mv tmp.json pc-resources-config.json

echo "Starting the node..."
expect <<EOF
spawn ./partner-chains-node wizards start-node
expect "Proceed? (Y/n)"
send "Y\r"
set timeout -1
expect eof
EOF

# tail -f /dev/null