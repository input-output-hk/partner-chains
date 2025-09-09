#!/bin/bash

# Script to upgrade multiple nodes with BEEFY-enabled binary
# Usage: ./upgrade-all-nodes.sh [node1] [node2] [node3] ...
# Example: ./upgrade-all-nodes.sh partner-chains-node-1 partner-chains-node-2

if [ $# -eq 0 ]; then
    echo "Usage: $0 [container-name1] [container-name2] ..."
    echo "Example: $0 partner-chains-node-1 partner-chains-node-2"
    echo ""
    echo "Available containers:"
    docker ps --format "table {{.Names}}\t{{.Image}}" | grep partner-chains-node
    exit 1
fi

SCRIPT_DIR=$(dirname "$0")

for container in "$@"; do
    echo ""
    echo "=== Processing $container ==="
    "$SCRIPT_DIR/upgrade-node-binary.sh" "$container"
    if [ $? -eq 0 ]; then
        echo "✓ Successfully upgraded $container"
    else
        echo "✗ Failed to upgrade $container"
    fi
done

echo ""
echo "=== All upgrades completed ==="
echo "Remember to restart containers for changes to take effect"
