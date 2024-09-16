#!/bin/bash

echo "Waiting for Cardano chain to start..."

while true; do
    if [ -f "/shared/cardano.ready" ]; then
        break
    else
        sleep 10
    fi
done

echo "Cardano chain ready. Starting DB-Sync..."

# Find the schema directory and remove migration-4-* files
schema_dir=$(find /nix/store -type d -name "*-schema" -print -quit)
if [ -n "$schema_dir" ]; then
    find "$schema_dir" -name "migration-4-*" -exec rm {} \;
else
    echo "Schema directory not found."
fi

# Find the entrypoint executable, make it executable, and run it
entrypoint_executable=$(find /nix/store -type f -path "*/bin/entrypoint" -print -quit)
if [ -n "$entrypoint_executable" ]; then
    chmod +x "$entrypoint_executable"
    exec "$entrypoint_executable" --config /shared/db-sync-config.json --socket-path /node-ipc/node.socket
else
    echo "Entrypoint executable not found."
fi