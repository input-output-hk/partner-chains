#!/bin/bash

echo "Installing dependencies..."
apt -qq update &> /dev/null
apt -qq -y install expect jq &> /dev/null
cp /usr/local/bin/partner-chains-node /data/partner-chains-node
cd /data


start_node() {
    echo "Starting the node..."
    expect <<EOF
spawn ./partner-chains-cli start-node
expect "Proceed? (Y/n)"
send "Y\r"
set timeout -1
expect eof
EOF
}


if [ -f "/shared/partner-chains-wizard-3.ready" ]; then
    echo "/shared/partner-chains-wizard-3.ready exists. Skipping configuration and copying chain-spec.json and partner-chains-cli-chain-config.json..."
    cp /shared/chain-spec.json /data/chain-spec.json
    cp /shared/partner-chains-cli-chain-config.json /data/partner-chains-cli-chain-config.json
    start_node
    exit 0
fi

# COMPATIBILITY WITH v1.4.0 (PC-CLI and PC-CONTRACTS-CLI)
ldd --version
apt update && apt install -y wget unzip curl

# Download and install nvm:
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
\. "$HOME/.nvm/nvm.sh"
nvm install 22
node -v # Should print "v22.14.0".
nvm current # Should print "v22.14.0".
npm -v # Should print "10.9.2".


# Initialize flags
PC_CLI_READY=0
PC_CONTRACTS_CLI_READY=0

# Check which artifacts need to be downloaded
if [ "$PC_CONTRACTS_CLI_READY" -eq 0 ]; then
  echo "Downloading pc-contracts-cli and node_modules..."
  wget -q -O ./pc-contracts-cli.zip "$PC_CONTRACTS_CLI_ZIP_URL"
  unzip -o ./pc-contracts-cli.zip > /dev/null
fi

if [ "$PC_CLI_READY" -eq 0 ]; then
  echo "Downloading partner-chains-cli..."
  wget -q -O ./partner-chains-cli "$PARTNER_CHAINS_CLI_URL"
fi

# Set executable permissions
chmod +x ./partner-chains-cli
chmod +x ./pc-contracts-cli

echo "Beginning configuration..."
echo "Generating keys..."
expect <<EOF
spawn ./partner-chains-cli generate-keys
set timeout 60
expect "node base path (./data)"
send ".\r"
expect eof
EOF


echo "Waiting for chain-spec.json and partner-chains-cli-chain-config.json to be ready..."
while true; do
    if [ -f "/shared/chain-spec.ready" ]; then
        break
    else
        sleep 1
    fi
done

cp /shared/chain-spec.json /data/chain-spec.json
cp /shared/partner-chains-cli-chain-config.json /data/partner-chains-cli-chain-config.json

echo "Waiting for governance to setup main chain state to avoid spending the same utxo..."
while true; do
    if [ -f "/shared/partner-chains-wizard-1.ready" ]; then
        break
    else
        sleep 1
    fi
done

echo "Registering candidate..."
register1_output=$(expect <<EOF
spawn ./partner-chains-cli register1
set timeout 60
expect "path to the payment verification file (payment.vkey)"
send "/keys/funded_address.vkey\r"
expect "Ogmios protocol (http/https)"
send "\r"
expect "Ogmios hostname (localhost)"
send "ogmios\r"
expect "Ogmios port (1337)"
send "\r"
expect "Select UTXO to use for registration"
send "\033\[B\r"
expect eof
catch wait result
exit [lindex \$result 3]
EOF
)

register2_command=$(echo "$register1_output" | sed -n '/\/partner-chains-cli register2 \\/,$p' | tr -d '\\\n')
echo "$register2_command"

echo "Running register2 command..."
register2_output=$(expect <<EOF
spawn $register2_command
set timeout 60
expect "Path to mainchain signing key file (cold.skey)"
send "/keys/cold.skey\r"
expect "/partner-chains-cli register3"
expect eof
catch wait result
exit [lindex \$result 3]
EOF
)

register3_command=$(echo "$register2_output" | sed -n '/\/partner-chains-cli register3 \\/,$p' | tr -d '\\\n')
echo "$register3_command"

echo "Running register3 command..."
expect <<EOF
spawn $register3_command
set timeout 300
expect "Path to mainchain payment signing key file (payment.skey)"
send "/keys/funded_address.skey\r"
expect "Kupo protocol (http/https)"
send "\r"
expect "Kupo hostname"
send "kupo\r"
expect "Kupo port"
send "\r"
expect "Ogmios protocol (http/https)"
send "\r"
expect "Ogmios hostname (ogmios)"
send "\r"
expect "Ogmios port (1337)"
send "\r"
expect "Show registration status? (Y/n)"
send "Y\r"
expect "DB-Sync Postgres connection string"
send "postgresql://postgres:$POSTGRES_PASSWORD@postgres:$POSTGRES_PORT/cexplorer\r"
expect eof
EOF

echo "Configuring Node P2P port..."
jq '.node_p2p_port = 30335' partner-chains-cli-resources-config.json > tmp.json && mv tmp.json partner-chains-cli-resources-config.json

touch /shared/partner-chains-wizard-3.ready
echo "Registration complete."

start_node
