{
  self,
  inputs,
  ...
}: {
  perSystem = {
    inputs',
    self',
    lib,
    pkgs,
    system,
    ...
  }: let
    mkStack = network: let
      data-dir = "./.run/${network}";
      node-socket = "${data-dir}/cardano-node/node.socket";
      configs-dir = "${inputs.configurations}/network/${network}";
      node-config = "${configs-dir}/cardano-node/config.json";
      magic = if network == "preview" then "2" else "1";
    in {
      "tip-status-${network}" = {
        namespace = network;
        command = ''
          export CARDANO_NODE_SOCKET_PATH=${node-socket}
          while true; do
            ${self'.packages.cardano-cli}/bin/cardano-cli \
              query tip --testnet-magic ${magic};
            sleep 10
          done
        '';
        depends_on = {
          "cardano-node-${network}".condition = "process_healthy";
        };

        readiness_probe = {
          exec = {
            command = ''
              export CARDANO_NODE_SOCKET_PATH=${node-socket}
              ${self'.packages.cardano-cli}/bin/cardano-cli \
              query tip --testnet-magic ${magic} \
              | jq -e '.syncProgress == "100.00" | not' && exit 1 || exit 0
            '';
          };
          initial_delay_seconds = 25;
          period_seconds = 30;
          timeout_seconds = 10;
          success_threshold = 1;
          failure_threshold = 1000;
        };
      };
      "cardano-node-${network}" = {
        namespace = network;
        liveness_probe = {
          exec = {
            command = ''
              pgrep -f cardano-node
            '';
          };
          initial_delay_seconds = 5;
          period_seconds = 2;
          timeout_seconds = 5;
          success_threshold = 5;
          failure_threshold = 3;
        };
        readiness_probe = {
          exec = {
            command = ''
              while true; do
                if  [ -S ${node-socket} ] && nc -U -z -w 1 ${node-socket}; then
                  exit 0
                fi
                sleep 5
              done
            '';
          };
          initial_delay_seconds = 25;
          period_seconds = 5;
          timeout_seconds = 20;
          success_threshold = 1;
          failure_threshold = 1000;
        };
        availability.restart = "on_failure";
        shutdown = {
          signal = 2;
        };
        command = ''
          ${self'.packages.cardano-node}/bin/cardano-node run +RTS -N -RTS \
          --topology ${configs-dir}/cardano-node/topology.json \
          --database-path ${data-dir}/cardano-node/data \
          --socket-path ${node-socket} \
          --host-addr 0.0.0.0 \
          --port ${if network == "preview" then "3030" else "3031"} \
          --config ${node-config}
        '';
        environment = {
          NETWORK = network;
          CARDANO_NODE_SOCKET_PATH = node-socket;
        };
      };
      "db-sync-${network}" = let
        pgpass = pkgs.writeText "pgpass-mainnet" ''
          127.0.0.1:${if network == "preview" then "5432" else "5433"}:cexplorer:postgres:password123
        '';
      in {
        namespace = network;
        command = pkgs.writeShellApplication {
          name = "cardano-db-sync";
          runtimeInputs = [pkgs.postgresql];
          text = ''
            ${self'.packages."cardano-db-sync:exe:cardano-db-sync"}/bin/cardano-db-sync \
              --config ${configs-dir}/cardano-db-sync/config.json \
              --socket-path ${node-socket} \
              --state-dir ${data-dir}/db-sync/ledger-state \
              --schema-dir  ${inputs.cardano-dbsync}/schema/
          '';
        };
        depends_on = {
          "postgres-${network}".condition = "process_healthy";
          "cardano-node-${network}".condition = "process_healthy";
        };
        liveness_probe = {
          exec = {
            command = ''
              pgrep -f cardano-db-sync
            '';
          };
        };
        availability.restart = "on_failure";
        environment = {
          NETWORK = network;
          PGPASSFILE = "${pgpass}";
          POSTGRES_HOST = "127.0.0.1";
          POSTGRES_PORT = if network == "preview" then "5432" else "5433";
          POSTGRES_DB = "cexplorer";
          POSTGRES_USER = "postgres";
          POSTGRES_PASSWORD = "password123";
        };
      };
      "ogmios-${network}" = {
        namespace = network;
        command = ''
          ${self'.packages.ogmios}/bin/ogmios \
            --host 0.0.0.0 \
            --node-config ${node-config} \
            --node-socket ${node-socket} \
            --port ${if network == "preview" then "1337" else "1338"}
        '';
        environment = {
          DATA_DIR = "${data-dir}/ogmios";
          OGMIOS_PORT = if network == "preview" then "1337" else "1338";
        };
        liveness_probe = {
          exec = {
            command = ''
              pgrep -f ogmios
            '';
          };
          initial_delay_seconds = 5;
          period_seconds = 2;
          timeout_seconds = 5;
          success_threshold = 5;
          failure_threshold = 3;
        };
        readiness_probe = {
          http_get = {
            host = "0.0.0.0";
            port = if network == "preview" then 1337 else 1338;
          };
          initial_delay_seconds = 5;
          period_seconds = 5;
          timeout_seconds = 20;
          success_threshold = 1;
          failure_threshold = 1000;
        };
        availability.restart = "on_failure";
        depends_on."cardano-node-${network}".condition = "process_healthy";
      };
      "kupo-${network}" = {
        namespace = network;
        command = ''
          ${self'.packages.kupo}/bin/kupo \
            --node-socket ${node-socket} \
            --node-config ${node-config} \
            --host 0.0.0.0 \
            --workdir ${data-dir}/kupo \
            --match "*" \
            --since origin \
            --port ${if network == "preview" then "1442" else "1443"}
        '';
        liveness_probe = {
          exec = {
            command = ''
              pgrep -f kupo
            '';
          };
          initial_delay_seconds = 5;
          period_seconds = 2;
          timeout_seconds = 5;
          success_threshold = 5;
          failure_threshold = 3;
        };
        readiness_probe = {
          http_get = {
            host = "0.0.0.0";
            port = if network == "preview" then 1442 else 1443;
            path = "/matches";
          };
          initial_delay_seconds = 5;
          period_seconds = 5;
          timeout_seconds = 20;
          success_threshold = 1;
          failure_threshold = 20;
        };
        availability.restart = "on_failure";
        depends_on."cardano-node-${network}".condition = "process_healthy";
      };
    };
    mkService = network: let
      data-dir = "./.run/${network}";
    in {
      "postgres-${network}" = {
        enable = true;
        namespace = network;
        port = if network == "preview" then 5432 else 5433;
        dataDir = "${data-dir}/db-sync/database";
        listen_addresses = "127.0.0.1";
        initialDatabases = [{name = "cexplorer";}];
        superuser = "postgres";
        initdbArgs = [
          "--locale=C"
          "--encoding=UTF8"
          "--username=postgres"
          "--pwfile=${pkgs.writeText "password" "password123"}"
        ];
      };
    };
  in {
    process-compose."partnerchains-stack-unwrapped" = {
      imports = [
        inputs.services-flake.processComposeModules.default
      ];
      tui = true;
      settings.processes = mkStack "preview" // mkStack "preprod";
      services.postgres = mkService "preview" // mkService "preprod";
    };
  };
}
