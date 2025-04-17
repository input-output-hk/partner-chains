#!/bin/bash

echo "Installing dependencies..."
apt -qq update &> /dev/null
apt -qq -y install expect jq &> /dev/null
cp /usr/local/bin/partner-chains-node /data/partner-chains-node
cd /data


if [ -f "/shared/partner-chains-wizard-2.ready" ]; then
    echo "/shared/partner-chains-wizard-2.ready exists. Skipping configuration and copying chain-spec.json and partner-chains-cli-chain-config.json..."
    cp /shared/chain-spec.json /data/chain-spec.json
    cp /shared/partner-chains-cli-chain-config.json /data/partner-chains-cli-chain-config.json
    echo "Starting the node..."
    expect <<EOF
spawn ./partner-chains-cli start-node
expect "Proceed? (Y/n)"
send "Y\r"
set timeout -1
expect eof
EOF
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
expect "All done!"
expect eof
EOF

cp partner-chains-public-keys.json /shared/partner-chains-public-keys.json
touch /shared/partner-chains-wizard-2-keys.ready


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

echo "Configuring Node P2P port..."
jq '.node_p2p_port = 30334' partner-chains-cli-resources-config.json > tmp.json && mv tmp.json partner-chains-cli-resources-config.json

touch /shared/partner-chains-wizard-2.ready
echo "Configuration complete."

echo "Starting the node..."
expect <<EOF
spawn ./partner-chains-cli start-node
expect "DB-Sync Postgres connection string (postgresql://postgres-user:postgres-password@localhost:5432/cexplorer)"
send "postgresql://postgres:$POSTGRES_PASSWORD@postgres:$POSTGRES_PORT/cexplorer\r"
expect "Proceed? (Y/n)"
send "Y\r"
set timeout -1
expect eof
EOF
