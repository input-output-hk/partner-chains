#!/bin/bash

# Initialize flags
PC_NODE_READY=0

if [ "$ARTIFACT_OVERRIDE" == "yes" ]; then
  echo "Artifact override is enabled. Checking for local artifacts..."

  # Check and set flags for existing artifacts, and copy if found
  if [ -f "/overrides/partner-chains-node" ]; then
    echo "partner-chains-node found in /overrides/. Using local artifact."
    cp /overrides/partner-chains-node ./partner-chains-node
    echo "partner-chains-node copied."
    PC_NODE_READY=1
  fi

else
  echo "Artifact override is not enabled. Defaulting to downloading all artifacts..."
fi

# Check which artifacts need to be downloaded
if [ "$PC_NODE_READY" -eq 0 ]; then
  echo "Downloading partner-chains-node..."
  wget -q -O ./partner-chains-node "$PARTNER_CHAINS_NODE_URL"
fi


# Set executable permissions
chmod +x ./partner-chains-node

# Install jq
apt -qq update &> /dev/null
apt -qq -y install jq ncat &> /dev/null

echo "Dependencies downloaded and binaries made executable."

echo -e "Container will now idle, but will remain available for accessing the partner-chains-node utility as follows:\n"
echo "docker exec partner-chains-node /partner-chains-node/partner-chains-node --help"

tail -f /dev/null
'
