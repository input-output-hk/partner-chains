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

echo -e "Container will now idle, but will remain available for accessing the pc-contracts-cli utility as follows:\n"
echo "docker exec pc-contracts-cli /pc-contracts-cli/pc-contracts-cli --help"

tail -f /dev/null
'
