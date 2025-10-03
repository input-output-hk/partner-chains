#!/bin/bash

# Manual BEEFY key generation script for partner-chains-node-1
# This script follows the exact steps you provided

set -e

NODE_NUMBER=${1:-1}
CONTAINER_NAME="partner-chains-node-${NODE_NUMBER}"

echo "🔑 Manual BEEFY key generation for ${CONTAINER_NAME}"
echo ""

# Step 1: Generate the key
echo "Step 1: Generating BEEFY key..."
echo "Running: partner-chains-node key generate --scheme ecdsa --output-type json"
echo ""

# Execute the command in the container
docker exec ${CONTAINER_NAME} /usr/local/bin/partner-chains-node key generate --scheme ecdsa --output-type json

echo ""
echo "Step 2: Create key identifier (hex encoding of 'beefy')..."
KEY_IDENTIFIER=$(echo -n "beefy" | xxd -p)
echo "Key identifier: ${KEY_IDENTIFIER}"

echo ""
echo "Step 3: Create the key file..."
echo "You need to replace the mnemonic phrase below with the actual phrase from Step 1"
echo ""
echo "Command to run:"
echo "echo \"\\\"YOUR_MNEMONIC_PHRASE_HERE\\\"\" > ~/.local/share/partner-chains-node/chains/partner_chains_template/keystore/${KEY_IDENTIFIER}"

echo ""
echo "Step 4: Set permissions..."
echo "Command to run:"
echo "chmod 600 ~/.local/share/partner-chains-node/chains/partner_chains_template/keystore/${KEY_IDENTIFIER}"

echo ""
echo "Step 5: Verify..."
echo "Commands to run:"
echo "ls -la ~/.local/share/partner-chains-node/chains/partner_chains_template/keystore/"
echo "cat ~/.local/share/partner-chains-node/chains/partner_chains_template/keystore/${KEY_IDENTIFIER}"

echo ""
echo "📝 Instructions:"
echo "1. Copy the mnemonic phrase from Step 1 output"
echo "2. Replace YOUR_MNEMONIC_PHRASE_HERE in the Step 3 command with the actual phrase"
echo "3. Run the commands in the container or on the host as needed"
echo ""
echo "🐳 To run commands in the container:"
echo "docker exec -it ${CONTAINER_NAME} /bin/sh"
