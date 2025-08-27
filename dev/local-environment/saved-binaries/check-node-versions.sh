#!/bin/bash

# Script to check versions of all partner-chains nodes
echo "=== Partner Chains Node Versions ==="
echo ""

for container in $(docker ps --format "{{.Names}}" | grep partner-chains-node | sort); do
    echo "Container: $container"
    version=$(docker exec "$container" partner-chains-node --version 2>/dev/null || echo "Failed to get version")
    echo "  Version: $version"
    echo ""
done
