#!/usr/bin/env bash

PARTNER_CHAINS_NODE_IMAGE="ghcr.io/input-output-hk/partner-chains/partner-chains-node:v1.1.1-rc1"
CARDANO_IMAGE="ghcr.io/intersectmbo/cardano-node:9.1.0"
DBSYNC_IMAGE="ghcr.io/intersectmbo/cardano-db-sync:13.3.0.0"
KUPO_IMAGE="cardanosolutions/kupo:v2.9.0"
OGMIOS_IMAGE="cardanosolutions/ogmios:v6.6.0"
POSTGRES_IMAGE="postgres:15.3"
SIDECHAIN_MAIN_CLI_IMAGE="node:22-bookworm"
PC_CONTRACTS_CLI_ZIP_URL="https://github.com/input-output-hk/partner-chains-smart-contracts/releases/download/v6.1.0/trustless-sidechain-cli-6.1.0-x86_64-linux.zip"
PARTNER_CHAINS_NODE_URL="https://github.com/input-output-hk/partner-chains/releases/download/1.1.1-rc1/partner-chains-node-1.1.1-rc1-x86_64-linux"
PARTNER_CHAINS_CLI_URL="https://github.com/input-output-hk/partner-chains/releases/download/1.1.1-rc1/partner-chains-cli-1.1.1-rc1-x86_64-linux"

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

configure_kupo() {
  echo "===== KUPO CONFIGURATION ========"
  read -p "Do you want to set a non-default port for Kupo? (Will default to 1442) (Y/N): " set_kupo_port
  if [[ $set_kupo_port == [Yy]* ]]; then
    kupo_port=$(validate_port "Enter port for Kupo: ")
  else
    kupo_port=1442
  fi
  echo
}

configure_artifact_overrides() {
    local mode=$1 

    if [ "$mode" == "interactive" ]; then
        echo "===== ARTIFACT OVERRIDE CONFIGURATION ========"

        if [ "$overrides" == "yes" ]; then
            echo -e "Artifact overrides enabled. \n"
            artifact_override=yes
        else
            read -p "Do you want to override artifacts from local paths? (Y/N): " override_artifact
            if [[ $override_artifact == [Yy]* ]]; then
                artifact_override=yes
                echo -e "Artifact overrides enabled. \n"
                echo "To override pc-contracts-cli artifact, copy artifacts to path:"
                echo -e "./configurations/pc-contracts-cli/overrides/pc-contracts-cli and ./configurations/pc-contracts-cli/overrides/node_modules \n"
                echo "To override the partner-chains-node artifact, copy artifact to path:"
                echo -e "./configurations/pc-contracts-cli/overrides/partner-chains-node \n"
                echo "To override the partner-chains-cli artifact, copy artifact to path:"
                echo -e "./configurations/pc-contracts-cli/overrides/partner-chains-cli \n"
            else
                artifact_override=no
                echo -e "Artifact overrides disabled. Stable versions will be automatically downloaded within the container from Github Releases. \n"
            fi
        fi
    else
        # Non-interactive mode
        if [ "$overrides" == "yes" ]; then
            echo -e "Artifact overrides enabled. \n"
            artifact_override=yes
        else
            artifact_override=no
        fi
    fi

    # Check for the existence of the artifact paths
    if [ "$artifact_override" == "yes" ]; then
        # Check for pc-contracts-cli artifact
        if [[ -f "./configurations/pc-contracts-cli/overrides/pc-contracts-cli" && -d "./configurations/pc-contracts-cli/overrides/node_modules" ]]; then
            echo -e "pc-contracts-cli and node_modules found. Override enabled. \n"
        elif [[ -f "./configurations/pc-contracts-cli/overrides/pc-contracts-cli" && ! -d "./configurations/pc-contracts-cli/overrides/node_modules" ]]; then
            echo -e "Error: 'pc-contracts-cli' found but 'node_modules' directory is missing. \n"
            exit 1
        elif [[ ! -f "./configurations/pc-contracts-cli/overrides/pc-contracts-cli" && -d "./configurations/pc-contracts-cli/overrides/node_modules" ]]; then
            echo -e "Error: 'node_modules' directory found but 'pc-contracts-cli' script is missing. \n"
            exit 1
        else
            echo -e "pc-contracts-cli and node_modules not found. Override disabled for pc-contracts-cli. \n"
        fi

        # Check for partner-chains-node artifact
        if [ -f "./configurations/pc-contracts-cli/overrides/partner-chains-node" ]; then
            echo -e "partner-chains-node found. Override enabled. \n"
        else
            echo -e "partner-chains-node not found. Override disabled for partner-chains-node. \n"
        fi

        # Check for partner-chains-cli artifact
        if [ -f "./configurations/pc-contracts-cli/overrides/partner-chains-cli" ]; then
            echo -e "partner-chains-cli found. Override enabled. \n"
        else
            echo -e "partner-chains-cli not found. Override disabled for partner-chains-cli. \n"
        fi
    fi
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
      cpu_kupo=0.4
      mem_kupo=600M
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
      cpu_kupo=$(validate_cpu_limit "Enter CPU limit for Kupo (e.g., 0.4 for 0.4 CPU): ")
      mem_kupo=$(validate_memory_limit "Enter Memory limit for Kupo (e.g., 600M for 600 MB): ")
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
    cpu_kupo=$DEFAULT_CPU_LIMIT
    mem_kupo=$DEFAULT_MEM_LIMIT
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
KUPO_PORT=1442
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
CPU_KUPO=0.000
MEM_KUPO=1000G
ARTIFACT_OVERRIDE=$artifact_override
EOF
    else
        cat <<EOF >.env
POSTGRES_PORT=$db_port
POSTGRES_PASSWORD=$db_password
OGMIOS_PORT=$ogmios_port
KUPO_PORT=$kupo_port
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
CPU_KUPO=$cpu_kupo
MEM_KUPO=$mem_kupo
ARTIFACT_OVERRIDE=$artifact_override
EOF
    fi

    cat <<EOF >>.env
CARDANO_IMAGE=$CARDANO_IMAGE
DBSYNC_IMAGE=$DBSYNC_IMAGE
KUPO_IMAGE=$KUPO_IMAGE
OGMIOS_IMAGE=$OGMIOS_IMAGE
POSTGRES_IMAGE=$POSTGRES_IMAGE
SIDECHAIN_MAIN_CLI_IMAGE=$SIDECHAIN_MAIN_CLI_IMAGE
PARTNER_CHAINS_NODE_IMAGE=${node_image:-$PARTNER_CHAINS_NODE_IMAGE}
PC_CONTRACTS_CLI_ZIP_URL=$PC_CONTRACTS_CLI_ZIP_URL
PARTNER_CHAINS_NODE_URL=$PARTNER_CHAINS_NODE_URL
PARTNER_CHAINS_CLI_URL=$PARTNER_CHAINS_CLI_URL
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
    echo "2) Include Cardano testnet with Kupo and Ogmios"
    echo "3) Include Cardano testnet, Kupo, Ogmios, DB-Sync and Postgres"
    read -p "Enter your choice (1/2/3): " deployment_option
  else
    deployment_option=4
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
        echo -e "Including Cardano testnet, Kupo, and Ogmios services.\n"
        cat ./modules/cardano.txt >> docker-compose.yml
        cat ./modules/ogmios.txt >> docker-compose.yml
        cat ./modules/kupo.txt >> docker-compose.yml
        ;;
      3)
        echo -e "Including Cardano testnet, Kupo, Ogmios, DB-Sync, and Postgres services.\n"
        cat ./modules/cardano.txt >> docker-compose.yml
        cat ./modules/ogmios.txt >> docker-compose.yml
        cat ./modules/kupo.txt >> docker-compose.yml
        cat ./modules/db-sync.txt >> docker-compose.yml
        cat ./modules/postgres.txt >> docker-compose.yml
        ;;
      *)
        echo -e "Including all services.\n"
        cat ./modules/cardano.txt >> docker-compose.yml
        cat ./modules/ogmios.txt >> docker-compose.yml
        cat ./modules/kupo.txt >> docker-compose.yml
        cat ./modules/db-sync.txt >> docker-compose.yml
        cat ./modules/postgres.txt >> docker-compose.yml
        cat ./modules/partner-chains-nodes.txt >> docker-compose.yml
        cat ./modules/pc-contracts-cli.txt >> docker-compose.yml
        ;;
    esac

    cat ./modules/volumes.txt >> docker-compose.yml
    echo -e "docker-compose.yml file created successfully.\n"
}

parse_arguments() {
    non_interactive=0
    deployment_option=4
    postgres_password=""
    overrides="no"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -n|--non-interactive)
                non_interactive=1
                shift
                ;;
            -d|--deployment-option)
                if [[ -n "$2" && "$2" =~ ^[123]$ ]]; then
                    deployment_option="$2"
                    shift 2
                else
                    echo "Error: Invalid deployment option '$2'. Valid options are 1, 2, or 3."
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
            -o|--overrides)
                overrides="yes"
                shift
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
                echo "  -d, --deployment-option   Specify one of the custom deployment options (1, 2, or 3)."
                echo "  -p, --postgres-password   Set a specific password for PostgreSQL (overrides automatic generation)."
                echo "  -o, --overrides           Enable custom artifact overrides from artifacts in ./configurations/pc-contracts-cli/ (PC and PCSC)."
                echo "  -i, --node-image          Specify a custom Partner Chains Node image."
                echo "  -h, --help                Display this help dialogue and exit."
                exit 0
                ;;
            --) # End of options
                shift
                break
                ;;
            *)
                echo "Invalid option: $1" 1>&2
                exit 1
                ;;
        esac
    done

    # Export variables for use in other functions
    export non_interactive
    export deployment_option
    export postgres_password
    export overrides
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
        configure_artifact_overrides "non-interactive"
        create_docker_compose "non-interactive"
    else
        display_banner
        detect_os "interactive"
        backup_files "interactive"
        configure_postgres "interactive"
        configure_ogmios
        configure_kupo
        configure_artifact_overrides "interactive"
        resource_limits_setup
        
        if [ "$deployment_option" -eq 4 ]; then
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



