#!/bin/bash

# Script to generate and insert BEEFY keys for partner-chains-node containers
# Usage: ./generate_beefy_keys.sh [node-number]

set -e

NODE_NUMBER=${1:-1}
CONTAINER_NAME="partner-chains-node-${NODE_NUMBER}"
KEYSTORE_PATH="/keystore"
KEYS_PATH="/Volumes/T7Shield/Users/larry/Projects/iohk/partner-chains/dev/local-environment/configurations/partner-chains-nodes/partner-chains-node-${NODE_NUMBER}/keystore"

echo "🔑 Generating BEEFY keys for ${CONTAINER_NAME}..."

# Step 1: Generate the BEEFY key using partner-chains-node
echo "Step 1: Generating BEEFY key..."
KEY_OUTPUT=$(docker exec ${CONTAINER_NAME} /usr/local/bin/partner-chains-node key generate --scheme ecdsa --output-type json 2>/dev/null)

if [ $? -ne 0 ]; then
    echo "❌ Failed to generate BEEFY key. Make sure the container is running and partner-chains-node is available."
    exit 1
fi

echo "✅ BEEFY key generated successfully"

# Step 2: Create key identifier (hex encoding of "beefy")
echo "Step 2: Creating key identifier..."
KEY_IDENTIFIER=$(echo -n "beefy" | xxd -p)
echo "Key identifier: ${KEY_IDENTIFIER}"

# Step 3: Extract the mnemonic phrase from the generated key
echo "Step 3: Extracting mnemonic phrase..."
MNEMONIC=$(echo "${KEY_OUTPUT}" | jq -r '.secretPhrase // .phrase // .mnemonic // empty')

if [ -z "${MNEMONIC}" ]; then
    echo "❌ Could not extract mnemonic phrase from generated key"
    echo "Key output: ${KEY_OUTPUT}"
    exit 1
fi

echo "Mnemonic phrase: ${MNEMONIC}"

# Step 4: Create the key file in the keystore
echo "Step 4: Creating keystore file..."
KEYSTORE_FILE="${KEYS_PATH}/${KEY_IDENTIFIER}"

# Create the keystore file with the mnemonic phrase
echo "\"${MNEMONIC}\"" > "${KEYSTORE_FILE}"

# Step 5: Set proper permissions
echo "Step 5: Setting permissions..."
chmod 600 "${KEYSTORE_FILE}"

# Step 6: Verify the setup
echo "Step 6: Verifying setup..."
echo "Keystore file created: ${KEYSTORE_FILE}"
echo "File permissions:"
ls -la "${KEYSTORE_FILE}"
echo "File contents:"
cat "${KEYSTORE_FILE}"

# Step 7: Also create the key files in the keys directory for reference
echo "Step 7: Creating reference key files..."
KEYS_DIR="/Volumes/T7Shield/Users/larry/Projects/iohk/partner-chains/dev/local-environment/configurations/partner-chains-nodes/partner-chains-node-${NODE_NUMBER}/keys"

# Extract public and private keys if available
PUBLIC_KEY=$(echo "${KEY_OUTPUT}" | jq -r '.publicKey // empty')
PRIVATE_KEY=$(echo "${KEY_OUTPUT}" | jq -r '.privateKey // .secretKey // empty')

if [ -n "${PUBLIC_KEY}" ]; then
    echo "${PUBLIC_KEY}" > "${KEYS_DIR}/beefy.vkey"
    echo "✅ Public key saved to ${KEYS_DIR}/beefy.vkey"
fi

if [ -n "${PRIVATE_KEY}" ]; then
    echo "${PRIVATE_KEY}" > "${KEYS_DIR}/beefy.skey"
    echo "✅ Private key saved to ${KEYS_DIR}/beefy.skey"
fi

echo ""
echo "🎉 BEEFY key setup completed successfully!"
echo ""
echo "Summary:"
echo "- Container: ${CONTAINER_NAME}"
echo "- Keystore file: ${KEYSTORE_FILE}"
echo "- Key identifier: ${KEY_IDENTIFIER}"
echo "- Mnemonic phrase: ${MNEMONIC}"
echo ""
echo "To verify the keys are working, you can check the keystore directory:"
echo "ls -la ${KEYS_PATH}/"
echo ""
echo "To restart the node with the new keys:"
echo "docker restart ${CONTAINER_NAME}"
