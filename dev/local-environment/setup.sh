#!/usr/bin/env bash

PARTNER_CHAINS_NODE_IMAGE="ghcr.io/input-output-hk/partner-chains/partner-chains-node:v1.4.0"
CARDANO_IMAGE="ghcr.io/intersectmbo/cardano-node:10.1.4"
DBSYNC_IMAGE="ghcr.io/intersectmbo/cardano-db-sync:13.6.0.4"
OGMIOS_IMAGE="cardanosolutions/ogmios:v6.11.0"
POSTGRES_IMAGE="postgres:17.2"
SIDECHAIN_MAIN_CLI_IMAGE="node:22-bookworm"
TESTS_IMAGE="python:3.10-slim"

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
  local pattern_mem='^[0-9]+[KMG]$'
  while true; do
    read -p "$1" mem_limit
    if [[ $mem_limit =~ $pattern_mem ]]; then
      echo "$mem_limit"
      break
    else
      echo "Invalid memory limit. Please enter a valid value (e.g., 500M for 500 MB, 2G for 2 GB)."
    fi
  done
}

configure_ogmios() {
  echo "===== OGMIOS CONFIGURATION ========"
  read -p "Do you want to set a non-default port for Ogmios? (Will default to 1337) (Y/N): " set_ogmios_port
  if [[ $set_ogmios_port == [Yy]* ]]; then
    ogmios_port=$(validate_port "Enter port for Ogmios: ")
  else
    ogmios_port=1337
  fi
  echo
}

resource_limits_setup() {
  echo "===== RESOURCE LIMITS SETUP ========"
  read -p "Do you want to restrict CPU and Memory limits for the stack? (Y/N) " restrict_resources
  if [[ $restrict_resources == [Yy]* ]]; then
    if [[ $cardano_node_enabled == true ]]; then
      read -p "Apply sensible limits (Total = 4 CPU / 4GB Memory)? (Y/N) " sensible_limits
    else
      read -p "Apply sensible limits (Total = 3 CPU / 3GB Memory)? (Y/N) " sensible_limits
    fi
    if [[ $sensible_limits == [Yy]* ]]; then
      CPU_PARTNER_CHAINS_NODE=0.4
      MEM_PARTNER_CHAINS_NODE=400M
      cpu_cardano=1
      mem_cardano=1000M
      cpu_postgres=0.5
      mem_postgres=500M
      cpu_dbsync=0.5
      mem_dbsync=200M
      cpu_ogmios=0.2
      mem_ogmios=500M
    else
      CPU_PARTNER_CHAINS_NODE=$(validate_cpu_limit "Enter CPU limit for Partner Chains nodes (e.g., 0.4 for 0.4 CPU): ")
      MEM_PARTNER_CHAINS_NODE=$(validate_memory_limit "Enter Memory limit for each of the 3 x Partner Chains nodes (e.g., 400M for 400 MB): ")

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

    if [ "$mode" == "non-interactive" ]; then
        db_port=5432
    else
        read -p "Do you want to set a non-default port for Postgres? (Will default to 5432) (Y/N): " set_db_port
        if [[ $set_db_port =~ ^[Yy] ]]; then
            db_port=$(validate_port "Enter port for Postgres: ")
        else
            db_port=5432
        fi
    fi

    db_host=postgres
}

configure_env() {
    local mode=$1

    if [ "$mode" == "interactive" ]; then
        echo "===== ENV FILE CREATION ============"
        echo "Creating new .env file with environment configuration..."
    fi

    if [ "$mode" == "non-interactive" ]; then
        cat <<EOF >.env
POSTGRES_PORT=5432
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
    else
        cat <<EOF >.env
POSTGRES_PORT=$db_port
POSTGRES_PASSWORD=$db_password
OGMIOS_PORT=$ogmios_port
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
    fi

    cat <<EOF >>.env
CARDANO_IMAGE=$CARDANO_IMAGE
DBSYNC_IMAGE=$DBSYNC_IMAGE
OGMIOS_IMAGE=$OGMIOS_IMAGE
POSTGRES_IMAGE=$POSTGRES_IMAGE
SIDECHAIN_MAIN_CLI_IMAGE=$SIDECHAIN_MAIN_CLI_IMAGE
TESTS_IMAGE=$TESTS_IMAGE
PARTNER_CHAINS_NODE_IMAGE=${node_image:-$PARTNER_CHAINS_NODE_IMAGE}
EOF

    if [ "$mode" == "interactive" ]; then
        echo -e ".env file created successfully.\n"
    fi
}

choose_deployment_option() {
  echo "===== CUSTOM STACK MODIFICATIONS ========"
  read -p "Make custom modification to the stack? (Y/N): " modify_stack
  if [[ $modify_stack =~ ^[Yy]$ ]]; then
    echo "Choose your deployment option:"
    echo "1) Include only Cardano testnet"
    echo "2) Include Cardano testnet with Ogmios"
    echo "3) Include Cardano testnet, Ogmios, DB-Sync and Postgres"
    echo "4) Deploy a single Partner Chains node with network_mode: "host" for external connections (adjust partner-chains-external-node.txt before running this script)"
    read -p "Enter your choice (1/2/3/4): " deployment_option
  else
    deployment_option=0
  fi
  echo
}

create_docker_compose() {
    local mode=$1

    if [ "$mode" == "interactive" ]; then
        echo "===== DOCKER-COMPOSE.YML CREATION ============"
        echo "Creating docker-compose.yml manifest file with service configurations."
    fi

    echo "services:" > docker-compose.yml

    case $deployment_option in
      1)
        echo -e "Including only Cardano testnet service.\n"
        cat ./modules/cardano.txt >> docker-compose.yml
        ;;
      2)
        echo -e "Including Cardano testnet, and Ogmios services.\n"
        cat ./modules/cardano.txt >> docker-compose.yml
        cat ./modules/ogmios.txt >> docker-compose.yml
        ;;
      3)
        echo -e "Including Cardano testnet, Ogmios, DB-Sync, and Postgres services.\n"
        cat ./modules/cardano.txt >> docker-compose.yml
        cat ./modules/ogmios.txt >> docker-compose.yml
        cat ./modules/db-sync.txt >> docker-compose.yml
        cat ./modules/postgres.txt >> docker-compose.yml
        ;;
      4)
        echo -e "Including all services with external partner chain node.\n"
        cat ./modules/cardano.txt >> docker-compose.yml
        cat ./modules/ogmios.txt >> docker-compose.yml
        cat ./modules/db-sync.txt >> docker-compose.yml
        cat ./modules/postgres.txt >> docker-compose.yml
        cat ./modules/partner-chains-external-node.txt >> docker-compose.yml
        cat ./modules/partner-chains-setup.txt >> docker-compose.yml
        ;;
      0)
        echo -e "Including all services.\n"
        cat ./modules/cardano.txt >> docker-compose.yml
        cat ./modules/ogmios.txt >> docker-compose.yml
        cat ./modules/db-sync.txt >> docker-compose.yml
        cat ./modules/postgres.txt >> docker-compose.yml
        cat ./modules/partner-chains-nodes.txt >> docker-compose.yml
        cat ./modules/partner-chains-setup.txt >> docker-compose.yml
        ;;
      *)
        echo "Invalid deployment option selected."
        exit 1
        ;;
    esac
    if [ "$tests_enabled" == "yes" ]; then
        echo -e "Including tests.\n"
        cat ./modules/tests.txt >> docker-compose.yml
    fi
    cat ./modules/volumes.txt >> docker-compose.yml
    echo -e "docker-compose.yml file created successfully.\n"
}

parse_arguments() {
    non_interactive=0
    deployment_option=0
    postgres_password=""
    tests_enabled="no"

    while [[ $# -gt 0 ]]; do
        case "$1" in
            -n|--non-interactive)
                non_interactive=1
                shift
                ;;
            -d|--deployment-option)
                if [[ -n "$2" && "$2" =~ ^[1-4]$ ]]; then
                    deployment_option="$2"
                    shift 2
                else
                    echo "Error: Invalid deployment option '$2'. Valid options are 1, 2, 3, or 4."
                    exit 1
                fi
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
            -t|--tests)
                tests_enabled="yes"
                echo "Tests enabled. Ensure contents of e2e-tests directory is copied to ./configurations/tests/."
                shift
                ;;
            -h|--help)
                echo "Usage: $0 [OPTION]..."
                echo "Initialize and configure the Docker environment."
                echo "  -n, --non-interactive     Run with no interactive prompts and accept sensible default configuration settings."
                echo "  -d, --deployment-option   Specify one of the custom deployment options (1, 2, 3, or 4)."
                echo "  -p, --postgres-password   Set a specific password for PostgreSQL (overrides automatic generation)."
                echo "  -i, --node-image          Specify a custom Partner Chains Node image."
                echo "  -t, --tests               Include tests container."
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
    export deployment_option
    export postgres_password
    export node_image
    export tests_enabled
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
        configure_ogmios
        resource_limits_setup

        if [ "$deployment_option" -eq 0 ]; then
            choose_deployment_option
        fi

        configure_env "interactive"
        create_docker_compose "interactive"
    fi

    echo "===== SETUP COMPLETE ======"
    echo -e "Run 'docker compose up -d' to deploy local network"
    echo -e "We recommend using 'lazydocker' or a similar Docker UI to monitor the network logs and performance"
    echo -e "Run 'docker compose down --volumes' when you wish to stop the network and delete all volumes"
}

main "$@"
