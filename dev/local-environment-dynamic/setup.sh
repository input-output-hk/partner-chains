#!/usr/bin/env bash

NUM_DBSYNC_INSTANCES=3
NUM_PERMISSIONED_NODES_TO_PROCESS=5
NUM_REGISTERED_NODES_TO_PROCESS=5

PARTNER_CHAINS_NODE_IMAGE="ghcr.io/input-output-hk/partner-chains/partner-chains-node-unstable:latest"
CARDANO_IMAGE="ghcr.io/intersectmbo/cardano-node:10.1.4"
DBSYNC_IMAGE="ghcr.io/intersectmbo/cardano-db-sync:13.6.0.5"
OGMIOS_IMAGE="cardanosolutions/ogmios:v6.12.0"
POSTGRES_IMAGE="postgres:17.2"

display_banner() {
  cat <<'EOF'
  ___                _      ___       _             _
 |_ _|_ _  _ __ _  _| |_   / _ \ _  _| |_ _ __ _  _| |_
  | || ' \| '_ \ || |  _| | (_) | || |  _| '_ \ || |  _|
 |___|_||_| .__/\_,_|\__|__\___/ \_,_|\__| .__/\_,_|\__|         _
 | |   ___|_|_ __ _| | | __|_ ___ _(_)_ _|_|_ _ _  _ __  ___ _ _| |_
 | |__/ _ \/ _/ _` | | | _|| ' \ V / | '_/ _ \ ' \| '  \/ -_) ' \  _|
 |____\___/\__\__,_|_| |___|_||_\_/|_|_| \___/_||_|_|_|_\___|_||_\__|

EOF
}

detect_os() {
    local mode=$1

    unameOut="$(uname -s)"
    archOut="$(uname -m)"
    case "${unameOut}" in
        Linux*) OS=Linux ;;
        Darwin*) OS=Mac ;;
        CYGWIN* | MINGW* | MSYS_NT*) OS=Windows ;;
        *) OS="UNKNOWN:${unameOut}" ;;
    esac

    if [ "$mode" != "non-interactive" ]; then
        echo "===== SYSTEM DETECTION ============"
        echo "Detected operating system: ${OS}"
        echo -e "Detected architecture: ${archOut}\n"
    fi

    if [[ ${OS} == "Windows" ]]; then
        [ "$mode" != "non-interactive" ] && echo -e "WARNING: This is untested on Windows, you may encounter emulation issues or syntax compatibilities \n"
    fi

    if [[ ${OS} == "Mac" && ${archOut} == "arm64" ]]; then
        export DOCKER_DEFAULT_PLATFORM=linux/amd64
        [ "$mode" != "non-interactive" ] && echo -e "Note: DOCKER_DEFAULT_PLATFORM has been set to linux/amd64 to enable emulation compatibility with Linux images on arm64 architecture.\n"
    fi
}

backup_files() {
    local mode=$1

    if [ -f ".env" ]; then
        if [ "$mode" == "interactive" ]; then
            echo "===== .ENV FILE CHECK ============"
            read -p "Found existing .env file. Backup and overwrite? (Y/N) " env_backup_choice
            if [[ $env_backup_choice =~ ^[Yy]$ ]]; then
                echo -e "Backing up existing .env file to .env.bak \n"
                mv .env .env.bak
            else
                echo "Exiting without changes to .env file."
                exit 1
            fi
        else
            mv .env .env.bak
        fi
    fi

    if [ -f "docker-compose.yml" ]; then
        if [ "$mode" == "interactive" ]; then
            echo "===== DOCKER-COMPOSE.YML FILE CHECK ============"
            read -p "Found existing docker-compose.yml file. Backup and overwrite? (Y/N) " compose_backup_choice
            if [[ $compose_backup_choice =~ ^[Yy]$ ]]; then
                echo -e "Backing up existing docker-compose.yml file to docker-compose.yml.bak \n"
                mv docker-compose.yml docker-compose.yml.bak
            else
                echo "Exiting without changes to docker-compose.yml file."
                exit 1
            fi
        else
            mv docker-compose.yml docker-compose.yml.bak
        fi
    fi
}

function validate_port() {
  while true; do
    read -p "$1" port
    if [[ $port =~ ^[0-9]{1,5}$ ]] && [ "$port" -ge 1 ] && [ "$port" -le 65535 ]; then
      echo "$port"
      break
    else
      echo "Invalid port. Please enter a port number between 1 and 65535."
    fi
  done
}

function validate_cpu_limit() {
  local pattern_cpu='^[0-9]+(\.[0-9]+)?$'
  while true; do
    read -p "$1" cpu_limit
    if [[ $cpu_limit =~ $pattern_cpu ]]; then
      echo "$cpu_limit"
      break
    else
      echo "Invalid CPU limit. Please enter a valid value (e.g., 0.5 for 0.5 CPU)."
    fi
  done
}

function validate_memory_limit() {
  local pattern_mem='^[0-9]+[kmg]$'
  while true; do
    read -p "$1" mem_limit
    if [[ $mem_limit =~ $pattern_mem ]]; then
      echo "$mem_limit"
      break
    else
      echo "Invalid memory limit. Please enter a valid value (e.g., 500m for 500 MB, 2g for 2 GB)."
    fi
  done
}

resource_limits_setup() {
  echo "===== RESOURCE LIMITS SETUP ========"
  read -p "Do you want to restrict CPU and Memory limits for the stack? (Y/N) " restrict_resources
  if [[ $restrict_resources == [Yy]* ]]; then
    if [[ $cardano_node_enabled == true ]]; then
      read -p "Apply sensible limits (Total = 32 CPU / 32GB Memory)? (Y/N) " sensible_limits
    else
      read -p "Apply sensible limits (Total = 31 CPU / 31GB Memory)? (Y/N) " sensible_limits
    fi
    if [[ $sensible_limits == [Yy]* ]]; then
      # Allocate 0.1 CPU and 100MB per node for 310 nodes
      CPU_PARTNER_CHAINS_NODE=0.1
      MEM_PARTNER_CHAINS_NODE=100M
      cpu_cardano=1
      mem_cardano=1000M
      cpu_postgres=0.5
      mem_postgres=500M
      cpu_dbsync=0.5
      mem_dbsync=200M
      cpu_ogmios=0.2
      mem_ogmios=500M
    else
      CPU_PARTNER_CHAINS_NODE=$(validate_cpu_limit "Enter CPU limit for each Partner Chains node (e.g., 0.1 for 0.1 CPU): ")
      MEM_PARTNER_CHAINS_NODE=$(validate_memory_limit "Enter Memory limit for each Partner Chains node (e.g., 100M for 100 MB): ")

      cpu_cardano=$(validate_cpu_limit "Enter CPU limit for Cardano node (e.g., 1 for 1 CPU): ")
      mem_cardano=$(validate_memory_limit "Enter Memory limit for Cardano node (e.g., 1000M for 1000 MB): ")

      cpu_postgres=$(validate_cpu_limit "Enter CPU limit for PostgreSQL database (e.g., 0.5 for 0.5 CPU): ")
      mem_postgres=$(validate_memory_limit "Enter Memory limit for PostgreSQL database (e.g., 500M for 500 MB): ")
      cpu_dbsync=$(validate_cpu_limit "Enter CPU limit for db-sync (e.g., 0.5 for 0.5 CPU): ")
      mem_dbsync=$(validate_memory_limit "Enter Memory limit for db-sync (e.g., 200M for 200 MB): ")
      cpu_ogmios=$(validate_cpu_limit "Enter CPU limit for Ogmios (e.g., 0.2 for 0.2 CPU): ")
      mem_ogmios=$(validate_memory_limit "Enter Memory limit for Ogmios (e.g., 500M for 500 MB): ")
    fi
  else
    DEFAULT_CPU_LIMIT="0.000"
    DEFAULT_MEM_LIMIT="1000G"
    CPU_PARTNER_CHAINS_NODE=$DEFAULT_CPU_LIMIT
    MEM_PARTNER_CHAINS_NODE=$DEFAULT_MEM_LIMIT
    cpu_cardano=$DEFAULT_CPU_LIMIT
    mem_cardano=$DEFAULT_MEM_LIMIT
    cpu_postgres=$DEFAULT_CPU_LIMIT
    mem_postgres=$DEFAULT_MEM_LIMIT
    cpu_dbsync=$DEFAULT_CPU_LIMIT
    mem_dbsync=$DEFAULT_MEM_LIMIT
    cpu_ogmios=$DEFAULT_CPU_LIMIT
    mem_ogmios=$DEFAULT_MEM_LIMIT
  fi
  echo
}

configure_postgres() {
    local mode=$1

    if [ "$mode" == "interactive" ]; then
        echo "===== POSTGRES CONFIGURATION ========"
    fi

    if [ "$mode" == "non-interactive" ]; then
        if [ -n "$postgres_password" ]; then
            db_password="$postgres_password"
        else
            db_password=$(LC_ALL=C tr -dc 'a-zA-Z0-9' < /dev/urandom | head -c 20)
        fi
    else
        if [ -n "$postgres_password" ]; then
            db_password="$postgres_password"
            echo "PostgreSQL password set via argument. Skipping password prompt."
        else
            read -p "Manually configure Postgres Password? (Will generate otherwise) (Y/N): " manual_configure
            if [[ $manual_configure =~ ^[Yy] ]]; then
                read -sp "Enter PostgreSQL postgres user password: " db_password
                echo
            else
                db_password=$(LC_ALL=C tr -dc 'a-zA-Z0-9' < /dev/urandom | head -c 20)
                echo -e "Generated PostgreSQL postgres user password: $db_password \n"
            fi
        fi
    fi

    for i in $(seq 1 $NUM_DBSYNC_INSTANCES); do
        eval "db_port_$i=$((5432 + i - 1))"
    done
}

configure_env() {
    local mode=$1

    if [ "$mode" == "interactive" ]; then
        echo "===== ENV FILE CREATION ============"
        echo "Creating new .env file with environment configuration..."
    fi

    if [ "$mode" == "non-interactive" ]; then
        cat <<EOF >.env
POSTGRES_PASSWORD=$db_password
OGMIOS_PORT=1337
CPU_PARTNER_CHAINS_NODE=0.000
MEM_PARTNER_CHAINS_NODE=1000G
CPU_CARDANO=0.000
MEM_CARDANO=1000G
CPU_POSTGRES=0.000
MEM_POSTGRES=1000G
CPU_DBSYNC=0.000
MEM_DBSYNC=1000G
CPU_OGMIOS=0.000
MEM_OGMIOS=1000G
EOF
        for i in $(seq 1 $NUM_DBSYNC_INSTANCES); do
            port_val=$(eval echo \$db_port_$i)
            echo "POSTGRES_PORT_$i=$port_val" >> .env
        done
    else
        cat <<EOF >.env
POSTGRES_PASSWORD=$db_password
OGMIOS_PORT=1337
CPU_PARTNER_CHAINS_NODE=$CPU_PARTNER_CHAINS_NODE
MEM_PARTNER_CHAINS_NODE=$MEM_PARTNER_CHAINS_NODE
CPU_CARDANO=$cpu_cardano
MEM_CARDANO=$mem_cardano
CPU_POSTGRES=$cpu_postgres
MEM_POSTGRES=$mem_postgres
CPU_DBSYNC=$cpu_dbsync
MEM_DBSYNC=$mem_dbsync
CPU_OGMIOS=$cpu_ogmios
MEM_OGMIOS=$mem_ogmios
EOF
        for i in $(seq 1 $NUM_DBSYNC_INSTANCES); do
            port_val=$(eval echo \$db_port_$i)
            echo "POSTGRES_PORT_$i=$port_val" >> .env
        done
    fi

    cat <<EOF >>.env
CARDANO_IMAGE=$CARDANO_IMAGE
DBSYNC_IMAGE=$DBSYNC_IMAGE
OGMIOS_IMAGE=$OGMIOS_IMAGE
POSTGRES_IMAGE=$POSTGRES_IMAGE
PARTNER_CHAINS_NODE_IMAGE=${node_image:-$PARTNER_CHAINS_NODE_IMAGE}
NUM_PERMISSIONED_NODES_TO_PROCESS=$NUM_PERMISSIONED_NODES_TO_PROCESS
NUM_REGISTERED_NODES_TO_PROCESS=$NUM_REGISTERED_NODES_TO_PROCESS
EOF

    if [ "$mode" == "interactive" ]; then
        echo -e ".env file created successfully.\n"
    fi
}

generate_node_configurations() {
    if [ "$NUM_PERMISSIONED_NODES_TO_PROCESS" -eq 0 ] && [ "$NUM_REGISTERED_NODES_TO_PROCESS" -eq 0 ]; then
        echo "ERROR: You must configure at least one permissioned or registered node."
        exit 1
    fi
    # Create directories for node configurations
    mkdir -p configurations/partner-chains-nodes

    if [ "$NUM_PERMISSIONED_NODES_TO_PROCESS" -gt 0 ]; then
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

# Create a local keystore and copy keys from the shared volume
echo "Creating local keystore and copying keys..."
mkdir /data/keystore
cp /shared/node-keys/$node_name/keystore/* /data/keystore/
chmod -R 777 /data/keystore

NODE_KEY='b0c7b085c8df4d8f0add881a39d90a0f29edd265dba1b9c2db5564f8e1b1a02a'
PEER_ID='12D3KooWD7ou3cgmVbMttbAXNvXPwna8LkRUko849YE8oWv5NGZP'

export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=\$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)

/usr/local/bin/partner-chains-node \\
  --chain=/shared/chain-spec.json \\
  --validator \\
  --node-key="\$NODE_KEY" \\
  --base-path=/data \\
  --keystore-path=/data/keystore \\
  --in-peers=10000 \\
  --out-peers=10000 \\
  --unsafe-rpc-external \\
  --rpc-port=9933 \\
  --rpc-cors=all \\
  --rpc-max-connections=10000 \\
  --prometheus-port=9615 \\
  --prometheus-external \\
  --state-pruning=archive \\
  --discover-local \\
  --blocks-pruning=archive \\
  --listen-addr=/ip4/0.0.0.0/tcp/30333 \\
  --public-addr="/dns4/partner-chains-node-permissioned-1/tcp/30333/p2p/\$PEER_ID" &

touch /shared/partner-chains-node-$node_name.ready

wait
EOF
        chmod +x configurations/partner-chains-nodes/$node_name/entrypoint.sh
    fi

    # Generate remaining permissioned nodes configurations
    for ((i=2; i<=NUM_PERMISSIONED_NODES_TO_PROCESS; i++)); do
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
echo "Staggering start by $i seconds..."
sleep $i

# Create a local keystore and copy keys from the shared volume
echo "Creating local keystore and copying keys..."
mkdir /data/keystore
cp /shared/node-keys/$node_name/keystore/* /data/keystore/
chmod -R 777 /data/keystore

NODE_KEY=\$(openssl rand -hex 32)
PEER_ID=\$(echo "\$NODE_KEY" | /usr/local/bin/partner-chains-node key inspect-node-key)

export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=\$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)

/usr/local/bin/partner-chains-node \\
  --chain=/shared/chain-spec.json \\
  --validator \\
  --node-key="\$NODE_KEY" \\
  --bootnodes="/dns/partner-chains-node-permissioned-1/tcp/30333/p2p/12D3KooWD7ou3cgmVbMttbAXNvXPwna8LkRUko849YE8oWv5NGZP" \\
  --base-path=/data \\
  --keystore-path=/data/keystore \\
  --in-peers=10000 \\
  --out-peers=10000 \\
  --unsafe-rpc-external \\
  --rpc-port=9933 \\
  --rpc-cors=all \\
  --rpc-max-connections=10000 \\
  --prometheus-port=9615 \\
  --prometheus-external \\
  --state-pruning=archive \\
  --discover-local \\
  --blocks-pruning=archive \\
  --listen-addr=/ip4/0.0.0.0/tcp/30333 \\
  --public-addr="/dns4/partner-chains-node-$node_name/tcp/30333/p2p/\$PEER_ID" &

touch /shared/partner-chains-node-$node_name.ready

wait
EOF
        chmod +x configurations/partner-chains-nodes/$node_name/entrypoint.sh
    done

    for ((i=1; i<=NUM_REGISTERED_NODES_TO_PROCESS; i++)); do
        node_name="registered-$i"
        mkdir -p configurations/partner-chains-nodes/$node_name
        
        # Check if this should be the bootnode
        is_bootnode=false
        if [ "$NUM_PERMISSIONED_NODES_TO_PROCESS" -eq 0 ] && [ "$i" -eq 1 ]; then
            is_bootnode=true
        fi

        # Generate entrypoint script
        if [ "$is_bootnode" = true ]; then
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

# Create a local keystore and copy keys from the shared volume
echo "Creating local keystore and copying keys..."
mkdir /data/keystore
cp /shared/node-keys/$node_name/keystore/* /data/keystore/
chmod -R 777 /data/keystore

NODE_KEY=\$(openssl rand -hex 32)
PEER_ID=\$(echo "\$NODE_KEY" | /usr/local/bin/partner-chains-node key inspect-node-key)

export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=\$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)

/usr/local/bin/partner-chains-node \\
  --chain=/shared/chain-spec.json \\
  --validator \\
  --node-key="\$NODE_KEY" \\
  --base-path=/data \\
  --keystore-path=/data/keystore \\
  --in-peers=10000 \\
  --out-peers=10000 \\
  --unsafe-rpc-external \\
  --rpc-port=9933 \\
  --rpc-cors=all \\
  --rpc-max-connections=10000 \\
  --prometheus-port=9615 \\
  --prometheus-external \\
  --state-pruning=archive \\
  --discover-local \\
  --blocks-pruning=archive \\
  --listen-addr=/ip4/0.0.0.0/tcp/30333 \\
  --public-addr="/dns4/partner-chains-node-$node_name/tcp/30333/p2p/\$PEER_ID" &

touch /shared/partner-chains-node-$node_name.ready

wait
EOF
        else
            # Determine bootnode address
            if [ "$NUM_PERMISSIONED_NODES_TO_PROCESS" -gt 0 ]; then
                bootnode_address="/dns/partner-chains-node-permissioned-1/tcp/30333/p2p/12D3KooWD7ou3cgmVbMttbAXNvXPwna8LkRUko849YE8oWv5NGZP"
            else
                # This assumes registered-1 is the bootnode
                # Note: This part of the script doesn't know the peer ID of registered-1 ahead of time.
                # This is a limitation. For a dynamic setup, a more robust discovery mechanism would be needed.
                # For now, we'll point to its service name and hope for the best.
                bootnode_address="/dns/partner-chains-node-registered-1/tcp/30333"
            fi
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
echo "Staggering start by $i seconds..."
sleep $i

# Create a local keystore and copy keys from the shared volume
echo "Creating local keystore and copying keys..."
mkdir /data/keystore
cp /shared/node-keys/$node_name/keystore/* /data/keystore/
chmod -R 777 /data/keystore

NODE_KEY=\$(openssl rand -hex 32)
PEER_ID=\$(echo "\$NODE_KEY" | /usr/local/bin/partner-chains-node key inspect-node-key)

export MC__FIRST_EPOCH_TIMESTAMP_MILLIS=\$(cat /shared/MC__FIRST_EPOCH_TIMESTAMP_MILLIS)

/usr/local/bin/partner-chains-node \\
  --chain=/shared/chain-spec.json \\
  --validator \\
  --node-key="\$NODE_KEY" \\
  --base-path=/data \\
  --keystore-path=/data/keystore \\
  --bootnodes="$bootnode_address" \\
  --in-peers=10000 \\
  --out-peers=10000 \\
  --unsafe-rpc-external \\
  --rpc-port=9933 \\
  --rpc-cors=all \\
  --rpc-max-connections=10000 \\
  --prometheus-port=9615 \\
  --prometheus-external \\
  --state-pruning=archive \\
  --discover-local \\
  --blocks-pruning=archive \\
  --listen-addr=/ip4/0.0.0.0/tcp/30333 \\
  --public-addr="/dns4/partner-chains-node-$node_name/tcp/30333/p2p/\$PEER_ID" &

touch /shared/partner-chains-node-$node_name.ready

wait
EOF
        fi
        chmod +x configurations/partner-chains-nodes/$node_name/entrypoint.sh
    done

    # Generate docker-compose.yml
    cat > docker-compose.yml <<EOF
services:
EOF

    batch_size=25
    last_batch_leader=""
    
    # Add permissioned nodes
    for ((i=1; i<=NUM_PERMISSIONED_NODES_TO_PROCESS; i++)); do
        node_name="permissioned-$i"
        rpc_port=$((11000 + i - 1))
        prometheus_port=$((9615 + i - 1))
        db_sync_instance=$(( (i - 1) % NUM_DBSYNC_INSTANCES + 1 ))
        
        cat >> docker-compose.yml <<EOF
  partner-chains-node-$node_name:
    container_name: partner-chains-node-$node_name
    image: \${PARTNER_CHAINS_NODE_IMAGE}
    platform: linux/amd64
    volumes:
      - shared-volume:/shared
      - partner-chains-node-$node_name-data:/data
      - ./configurations/partner-chains-nodes/$node_name/entrypoint.sh:/entrypoint.sh
EOF
        # Add dependency if a previous batch leader exists
        if [ -n "$last_batch_leader" ]; then
            cat >> docker-compose.yml <<EOF
    depends_on:
      $last_batch_leader:
        condition: service_healthy
EOF
        fi

        # If this node is the start of a new batch, it becomes the new leader
        if (( (i - 1) % batch_size == 0 )); then
            last_batch_leader="partner-chains-node-$node_name"
            # Add a healthcheck to this leader node
            cat >> docker-compose.yml <<EOF
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:9615/metrics"]
      interval: 10s
      timeout: 5s
      retries: 10
EOF
        fi

        cat >> docker-compose.yml <<EOF
    environment:
      DB_SYNC_POSTGRES_CONNECTION_STRING: "postgres://postgres:\${POSTGRES_PASSWORD}@postgres-${db_sync_instance}:5432/cexplorer"
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
    for ((i=1; i<=NUM_REGISTERED_NODES_TO_PROCESS; i++)); do
        node_name="registered-$i"
        rpc_port=$((11000 + i - 1 + NUM_PERMISSIONED_NODES_TO_PROCESS))
        prometheus_port=$((9615 + i - 1 + NUM_PERMISSIONED_NODES_TO_PROCESS))
        db_sync_instance=$(( (i - 1 + NUM_PERMISSIONED_NODES_TO_PROCESS) % NUM_DBSYNC_INSTANCES + 1 ))
        
        cat >> docker-compose.yml <<EOF
  partner-chains-node-$node_name:
    container_name: partner-chains-node-$node_name
    image: \${PARTNER_CHAINS_NODE_IMAGE}
    platform: linux/amd64
    volumes:
      - shared-volume:/shared
      - partner-chains-node-$node_name-data:/data
      - ./configurations/partner-chains-nodes/$node_name/entrypoint.sh:/entrypoint.sh
EOF
        # Add dependency if a previous batch leader exists
        if [ -n "$last_batch_leader" ]; then
            cat >> docker-compose.yml <<EOF
    depends_on:
      $last_batch_leader:
        condition: service_healthy
EOF
        fi

        # If this node is the start of a new batch, it becomes the new leader
        current_total_index=$((NUM_PERMISSIONED_NODES_TO_PROCESS + i))
        if (( (current_total_index - 1) % batch_size == 0 )); then
            last_batch_leader="partner-chains-node-$node_name"
            # Add a healthcheck to this leader node
            cat >> docker-compose.yml <<EOF
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:9615/metrics"]
      interval: 10s
      timeout: 5s
      retries: 10
EOF
        fi

        cat >> docker-compose.yml <<EOF
    environment:
      DB_SYNC_POSTGRES_CONNECTION_STRING: "postgres://postgres:\${POSTGRES_PASSWORD}@postgres-${db_sync_instance}:5432/cexplorer"
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

    echo "Generated node configurations and docker-compose.yml"
}

create_docker_compose() {
    local mode=$1
    local script_dir; script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    if [ "$mode" == "interactive" ]; then
        echo "===== DOCKER-COMPOSE.YML CREATION ============"
        echo "Creating docker-compose.yml manifest file with service configurations."
    fi

    # Generate node configurations and docker-compose.yml
    generate_node_configurations

    # Add other services
    cat "$script_dir/modules/cardano.txt" >> docker-compose.yml
    cat "$script_dir/modules/ogmios.txt" >> docker-compose.yml
    for i in $(seq 1 $NUM_DBSYNC_INSTANCES); do
        cat >> docker-compose.yml <<EOF
  postgres-${i}:
    container_name: postgres-${i}
    image: \${POSTGRES_IMAGE}
    platform: linux/amd64
    command: postgres -c max_connections=10000 -c maintenance_work_mem=256MB
    environment:
      POSTGRES_PASSWORD: \${POSTGRES_PASSWORD}
      POSTGRES_DB: cexplorer
      POSTGRES_MULTIPLE_DATABASES: cexplorer
    volumes:
      - postgres-data-${i}:/var/lib/postgresql/data
      - ./configurations/postgres/entrypoint.sh:/usr/local/bin/custom-entrypoint.sh
      - ./configurations/postgres/init.sh:/docker-entrypoint-initdb.d/init.sh
    ports:
      - "\${POSTGRES_PORT_${i}}:5432"
    restart: always
    deploy:
      resources:
        limits:
          cpus: \${CPU_POSTGRES:-}
          memory: \${MEM_POSTGRES:-}
  db-sync-${i}:
    container_name: db-sync-${i}
    image: \${DBSYNC_IMAGE}
    platform: linux/amd64
    entrypoint: ["/bin/bash", "/entrypoint.sh"]
    depends_on:
      - postgres-${i}
      - cardano-node-1
    volumes:
      - db-sync-data-${i}:/var/lib/cdbsync
      - shared-volume:/shared
      - cardano-node-1-data:/node-ipc
      - ./configurations/db-sync/entrypoint.sh:/entrypoint.sh
    environment:
      POSTGRES_HOST: postgres-${i}
      POSTGRES_PORT: 5432
      POSTGRES_DB: cexplorer
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: \${POSTGRES_PASSWORD}
    restart: always
    deploy:
      resources:
        limits:
          cpus: \${CPU_DBSYNC:-}
          memory: \${MEM_DBSYNC:-}
EOF
    done
    cat "$script_dir/modules/partner-chains-setup.txt" >> docker-compose.yml

    # Add volumes
    cat "$script_dir/modules/volumes.txt" >> docker-compose.yml
    for i in $(seq 1 $NUM_DBSYNC_INSTANCES); do
        echo "  postgres-data-${i}:" >> docker-compose.yml
        echo "  db-sync-data-${i}:" >> docker-compose.yml
    done
    echo "" >> docker-compose.yml # Ensure a newline

    # Add volume entries for all nodes
    for ((i=1; i<=NUM_PERMISSIONED_NODES_TO_PROCESS; i++)); do
        echo "  partner-chains-node-permissioned-$i-data:" >> docker-compose.yml
    done

    for ((i=1; i<=NUM_REGISTERED_NODES_TO_PROCESS; i++)); do
        echo "  partner-chains-node-registered-$i-data:" >> docker-compose.yml
    done

    cat <<EOF >>docker-compose.yml

networks:
  partner-chain-network:
    driver: bridge
EOF

    if [ "$mode" != "non-interactive" ]; then
      echo -e "docker-compose.yml file created successfully.\n"
    fi
}

parse_arguments() {
    non_interactive=0
    postgres_password=""

    while [[ $# -gt 0 ]]; do
        case "$1" in
            -n|--non-interactive)
                non_interactive=1
                shift
                ;;
            -p|--postgres-password)
                if [[ -n "$2" ]]; then
                    postgres_password="$2"
                    shift 2
                else
                    echo "Error: --postgres-password requires a value."
                    exit 1
                fi
                ;;
            -i|--node-image)
                if [[ -n "$2" ]]; then
                    node_image="$2"
                    shift 2
                else
                    echo "Error: --node-image requires a value."
                    exit 1
                fi
                ;;
            -h|--help)
                echo "Usage: $0 [OPTION]..."
                echo "Initialize and configure the Docker environment."
                echo "  -n, --non-interactive     Run with no interactive prompts and accept sensible default configuration settings."
                echo "  -p, --postgres-password   Set a specific password for PostgreSQL (overrides automatic generation)."
                echo "  -i, --node-image          Specify a custom Partner Chains Node image."
                echo "  -h, --help                Display this help dialogue and exit."
                exit 0
                ;;
            --)
                shift
                break
                ;;
            *)
                echo "Invalid option: $1" 1>&2
                exit 1
                ;;
        esac
    done

    export non_interactive
    export postgres_password
    export node_image
}

main() {
    parse_arguments "$@"

    if [ "$non_interactive" -eq 1 ]; then
        echo -e "Running in non-interactive mode with default settings...\n"
        backup_files "non-interactive"
        configure_postgres "non-interactive"
        detect_os "non-interactive"
        configure_env "non-interactive"
        create_docker_compose "non-interactive"
    else
        display_banner
        detect_os "interactive"
        backup_files "interactive"
        configure_postgres "interactive"
        resource_limits_setup
        configure_env "interactive"
        create_docker_compose "interactive"
    fi

    echo "===== SETUP COMPLETE ======"
    echo -e "Run 'bash setup.sh --non-interactive' to run the setup in non-interactive mode"
    echo -e "Run 'docker compose up -d' to deploy local network"
    echo -e "Use 'docker logs cardano-node-1 -f | grep -E \"DEBUG|LOG|WARN\"' to monitor the mainchain logs"
    echo -e "We recommend using 'lazydocker' or a similar Docker UI to monitor the network logs and performance"
    echo -e "Run 'docker compose down --volumes' when you wish to stop the network and delete all volumes"
}

main "$@"
