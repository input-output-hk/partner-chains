#!/bin/bash

# Helper script to upgrade a partner-chains node with the BEEFY-enabled binary
# Usage: ./upgrade-node-binary.sh <container-name>

if [ $# -ne 1 ]; then
    echo "Usage: $0 <container-name>"
    echo "Example: $0 partner-chains-node-1"
    exit 1
fi

CONTAINER_NAME="$1"
BINARY_PATH="./saved-binaries/partner-chains-node-beefy"

echo "=== Upgrading $CONTAINER_NAME with BEEFY-enabled binary ==="

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: BEEFY binary not found at $BINARY_PATH"
    exit 1
fi

# Check if container exists and is running
if ! docker ps --format "{{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
    echo "Error: Container $CONTAINER_NAME is not running"
    exit 1
fi

echo "Backing up original binary..."
docker exec "$CONTAINER_NAME" cp /usr/local/bin/partner-chains-node /usr/local/bin/partner-chains-node.backup

echo "Copying BEEFY-enabled binary to container..."
docker cp "$BINARY_PATH" "$CONTAINER_NAME":/usr/local/bin/partner-chains-node

echo "Setting permissions..."
docker exec "$CONTAINER_NAME" chmod +x /usr/local/bin/partner-chains-node

echo "Verifying upgrade..."
echo "New version:"
docker exec "$CONTAINER_NAME" /usr/local/bin/partner-chains-node --version

echo "Backup version:"
docker exec "$CONTAINER_NAME" /usr/local/bin/partner-chains-node.backup --version

echo ""
echo "=== Upgrade completed for $CONTAINER_NAME ==="
echo "Note: You'll need to restart the container for changes to take effect"
echo "To restart: docker restart $CONTAINER_NAME"
