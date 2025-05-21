#!/bin/bash

# Create directories for node configurations
mkdir -p configurations/partner-chains-nodes

# Generate permissioned-1 (bootnode) configuration
node_name="permissioned-1"
mkdir -p configurations/partner-chains-nodes/$node_name
cat > configurations/partner-chains-nodes/$node_name/entrypoint.sh <<EOF
#!/bin/sh

echo 'Waiting for Cardano chain to sync and Partner Chains smart contracts setup to complete...'

while true; do
    if [ -f "/shared/partner-chains-setup.ready" ]; then
        break
    else
        sleep 1
    fi
done

echo "Partner Chains smart contracts setup complete. Starting node..."

export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=\$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)

/usr/local/bin/partner-chains-node \\
  --chain=/shared/chain-spec.json \\
  --validator \\
  --node-key=0000000000000000000000000000000000000000000000000000000000000001 \\
  --base-path=/data \\
  --keystore-path=/keystore \\
  --unsafe-rpc-external \\
  --rpc-port=9933 \\
  --rpc-cors=all \\
  --prometheus-port=9615 \\
  --prometheus-external \\
  --state-pruning=archive \\
  --blocks-pruning=archive &

touch /shared/partner-chains-node-$node_name.ready

wait
EOF
chmod +x configurations/partner-chains-nodes/$node_name/entrypoint.sh

# Generate remaining permissioned nodes configurations
for i in {2..10}; do
    node_name="permissioned-$i"
    mkdir -p configurations/partner-chains-nodes/$node_name
    cat > configurations/partner-chains-nodes/$node_name/entrypoint.sh <<EOF
#!/bin/sh

echo 'Waiting for Cardano chain to sync and Partner Chains smart contracts setup to complete...'

while true; do
    if [ -f "/shared/partner-chains-setup.ready" ]; then
        break
    else
        sleep 1
    fi
done

echo "Partner Chains smart contracts setup complete. Starting node..."

export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=\$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)

/usr/local/bin/partner-chains-node \\
  --chain=/shared/chain-spec.json \\
  --validator \\
  --node-key=000000000000000000000000000000000000000000000000000000000000000$i \\
  --bootnodes="/dns/partner-chains-node-permissioned-1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \\
  --base-path=/data \\
  --keystore-path=/keystore \\
  --unsafe-rpc-external \\
  --rpc-port=9933 \\
  --rpc-cors=all \\
  --prometheus-port=9615 \\
  --prometheus-external \\
  --state-pruning=archive \\
  --blocks-pruning=archive &

touch /shared/partner-chains-node-$node_name.ready

wait
EOF
    chmod +x configurations/partner-chains-nodes/$node_name/entrypoint.sh
done

for i in {1..300}; do
    node_name="registered-$i"
    mkdir -p configurations/partner-chains-nodes/$node_name
    cat > configurations/partner-chains-nodes/$node_name/entrypoint.sh <<EOF
#!/bin/sh

echo 'Waiting for Cardano chain to sync and Partner Chains smart contracts setup to complete...'

while true; do
    if [ -f "/shared/partner-chains-setup.ready" ]; then
        break
    else
        sleep 1
    fi
done

echo "Partner Chains smart contracts setup complete. Starting node..."

export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=\$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)

/usr/local/bin/partner-chains-node \\
  --chain=/shared/chain-spec.json \\
  --validator \\
  --node-key=0000000000000000000000000000000000000000000000000000000000000$(printf "%03d" $i) \\
  --bootnodes="/dns/partner-chains-node-permissioned-1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \\
  --base-path=/data \\
  --keystore-path=/keystore \\
  --unsafe-rpc-external \\
  --rpc-port=9933 \\
  --rpc-cors=all \\
  --prometheus-port=9615 \\
  --prometheus-external \\
  --state-pruning=archive \\
  --blocks-pruning=archive &

touch /shared/partner-chains-node-$node_name.ready

wait
EOF
    chmod +x configurations/partner-chains-nodes/$node_name/entrypoint.sh
done

# Generate docker-compose.yml
cat > docker-compose.yml <<EOF
version: '3.8'

services:
EOF

# Add permissioned nodes
for i in {1..10}; do
    node_name="permissioned-$i"
    rpc_port=$((9933 + i - 1))
    prometheus_port=$((9615 + i - 1))
    
    cat >> docker-compose.yml <<EOF
  partner-chains-node-$node_name:
    container_name: partner-chains-node-$node_name
    image: \${PARTNER_CHAINS_NODE_IMAGE}
    platform: linux/amd64
    volumes:
      - shared-volume:/shared
      - partner-chains-node-$node_name-data:/data
      - ./configurations/partner-chains-nodes/$node_name/entrypoint.sh:/entrypoint.sh
    environment:
      DB_SYNC_POSTGRES_CONNECTION_STRING: "postgres://postgres:\${POSTGRES_PASSWORD}@postgres:\${POSTGRES_PORT}/cexplorer"
      CARDANO_SECURITY_PARAMETER: "5"
      CARDANO_ACTIVE_SLOTS_COEFF: "0.4"
      MC__FIRST_EPOCH_NUMBER: "0"
      MC__FIRST_SLOT_NUMBER: "0"
      MC__EPOCH_DURATION_MILLIS: "120000"
      BLOCK_STABILITY_MARGIN: "0"
    entrypoint: ["/bin/bash", "/entrypoint.sh"]
    ports:
      - "$rpc_port:9933"
      - "$prometheus_port:9615"
    restart: always
    deploy:
      resources:
        limits:
          cpus: \${CPU_PARTNER_CHAINS_NODE:-}
          memory: \${MEM_PARTNER_CHAINS_NODE:-}
EOF
done

# Add registered nodes
for i in {1..300}; do
    node_name="registered-$i"
    rpc_port=$((9943 + i - 1))
    prometheus_port=$((9625 + i - 1))
    
    cat >> docker-compose.yml <<EOF
  partner-chains-node-$node_name:
    container_name: partner-chains-node-$node_name
    image: \${PARTNER_CHAINS_NODE_IMAGE}
    platform: linux/amd64
    volumes:
      - shared-volume:/shared
      - partner-chains-node-$node_name-data:/data
      - ./configurations/partner-chains-nodes/$node_name/entrypoint.sh:/entrypoint.sh
    environment:
      DB_SYNC_POSTGRES_CONNECTION_STRING: "postgres://postgres:\${POSTGRES_PASSWORD}@postgres:\${POSTGRES_PORT}/cexplorer"
      CARDANO_SECURITY_PARAMETER: "5"
      CARDANO_ACTIVE_SLOTS_COEFF: "0.4"
      MC__FIRST_EPOCH_NUMBER: "0"
      MC__FIRST_SLOT_NUMBER: "0"
      MC__EPOCH_DURATION_MILLIS: "120000"
      BLOCK_STABILITY_MARGIN: "0"
    entrypoint: ["/bin/bash", "/entrypoint.sh"]
    ports:
      - "$rpc_port:9933"
      - "$prometheus_port:9615"
    restart: always
    deploy:
      resources:
        limits:
          cpus: \${CPU_PARTNER_CHAINS_NODE:-}
          memory: \${MEM_PARTNER_CHAINS_NODE:-}
EOF
done

# Add volumes
cat >> docker-compose.yml <<EOF

volumes:
  shared-volume:
  cardano-node-1-data:
EOF

# Add volume entries for all nodes
for i in {1..10}; do
    echo "  partner-chains-node-permissioned-$i-data:" >> docker-compose.yml
done

for i in {1..300}; do
    echo "  partner-chains-node-registered-$i-data:" >> docker-compose.yml
done

echo "Generated node configurations and docker-compose.yml" 