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
    data-dir = "./.run";
    node-socket = "${data-dir}/cardano-node/node.socket";
    configs-dir = "${inputs.configurations}/network/preview";
    node-config = "${configs-dir}/cardano-node/config.json";
  in {
    process-compose."partnerchains-stack-unwrapped" = {
      imports = [
        inputs.services-flake.processComposeModules.default
      ];
      tui = true;
      #port = 8081;
      services.postgres."postgres" = {
        enable = true;
        port = 5432;
        dataDir = "${data-dir}/db-sync/database";
        listen_addresses = "127.0.0.1";
        initialDatabases = [{name = "cexplorer";}];
        initdbArgs = [
          "--locale=C"
          "--encoding=UTF8"
          "--username=postgres"
          "--pwfile=${pkgs.writeText "password" "password123"}"
        ];
      };
      settings = {
        processes = {
          tip-status = {
            namespace = "cardano-node";
            command = ''
              export CARDANO_NODE_SOCKET_PATH=${node-socket}
              while true; do
                ${self'.packages.cardano-cli}/bin/cardano-cli \
                  query tip --testnet-magic 2;
                sleep 10
              done
            '';
            #is_tty = true;
            depends_on = {
              cardano-node.condition = "process_healthy";
            };

            readiness_probe = {
              exec = {
                command = ''
                  export CARDANO_NODE_SOCKET_PATH=${node-socket}
                  ${self'.packages.cardano-cli}/bin/cardano-cli \
                  query tip --testnet-magic 2 \
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
          cardano-node = {
            namespace = "cardano-node";
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
              --port 3030 \
              --config ${node-config}
            '';
            environment = {
              NETWORK = "preview";
              CARDANO_NODE_SOCKET_PATH = node-socket;
            };
          };
          db-sync = let
            pgpass = pkgs.writeText "pgpass-mainnet" ''
              127.0.0.1:5432:cexplorer:postgres:password123
            '';
          in {
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
              "postgres".condition = "process_healthy";
              cardano-node.condition = "process_healthy";
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
              NETWORK = "preview";
              PGPASSFILE = "${pgpass}";
              POSTGRES_HOST = "127.0.0.1";
              POSTGRES_PORT = "5432";
              POSTGRES_DB = "cexplorer";
              POSTGRES_USER = "postgres";
              POSTGRES_PASSWORD = "password123";
            };
          };
          ogmios = {
            command = ''
              ${self'.packages.ogmios}/bin/ogmios \
                --host 0.0.0.0 --node-config ${node-config} --node-socket ${node-socket}
          #   '';
            environment = {
              DATA_DIR = "${data-dir}/ogmios";
              OGMIOS_PORT = "1337";
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
                port = 1337;
              };
              initial_delay_seconds = 5;
              period_seconds = 5;
              timeout_seconds = 20;
              success_threshold = 1;
              failure_threshold = 1000;
            };
            availability.restart = "on_failure";
            depends_on.cardano-node.condition = "process_healthy";
          };
          kupo = {
            command = ''
              ${self'.packages.kupo}/bin/kupo \
                --node-socket ${node-socket} \
                --node-config ${node-config} \
                --host 0.0.0.0 \
                --workdir ${data-dir}/kupo \
                --match "*" \
                --since origin
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
                port = 1442;
                path = "/matches";
              };
              initial_delay_seconds = 5;
              period_seconds = 5;
              timeout_seconds = 20;
              success_threshold = 1;
              failure_threshold = 20;
            };
            availability.restart = "on_failure";
            depends_on.cardano-node.condition = "process_healthy";
          };
        };
      };
    };
  };
}
