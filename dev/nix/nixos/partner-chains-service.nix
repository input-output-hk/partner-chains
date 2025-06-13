# Partner Chains Node NixOS module
{self, ...}:
{
  flake.nixosModules.partner-chains =
    { config, lib, pkgs, ... }:

    with lib;

    let
      cfg = config.services.partner-chains;
    in {
      options.services.partner-chains = {
        enable = mkEnableOption "Partner Chains Node";
        
        # Common environment variables
        environment = mkOption {
          type = types.attrsOf types.str;
          default = {
            CARDANO_SECURITY_PARAMETER = "432";
            CARDANO_ACTIVE_SLOTS_COEFF = "0.05";
            DB_SYNC_POSTGRES_CONNECTION_STRING = "postgresql://cexplorer:password@localhost:5432/cexplorer";
            MC__FIRST_EPOCH_TIMESTAMP_MILLIS = "1666656000000";
            MC__EPOCH_DURATION_MILLIS = "86400000";
            MC__SLOT_DURATION_MILLIS = "1000";
            MC__FIRST_EPOCH_NUMBER = "0";
            MC__FIRST_SLOT_NUMBER = "0";
            BLOCK_STABILITY_MARGIN = "0";
          };
          description = "Environment variables for the partner chains node";
        };
        
        # Node-specific options
        nodeName = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "alice";
          description = "Node name flag (--alice, --bob, --charlie, etc.)";
        };
        
        nodeKey = mkOption {
          type = types.str;
          example = "0a04cb23cff606facb13ddd43655840e9f6f32bd7b432809620d461596e188e9";
          description = "Node key for identification";
        };
        
        chainSpecPath = mkOption {
          type = types.path;
          default = "/var/lib/partner-chains/chain-spec.json";
          description = "Path to the chain specification file";
        };
        
        keystorePath = mkOption {
          type = types.path;
          default = "/var/lib/partner-chains/keystore";
          description = "Path to the keystore directory";
        };
        
        listenAddr = mkOption {
          type = types.str;
          default = "/ip4/0.0.0.0/tcp/30333";
          description = "Address to listen on";
        };
        
        prometheusPort = mkOption {
          type = types.int;
          default = 9615;
          description = "Prometheus metrics port";
        };
        
        rpcPort = mkOption {
          type = types.int;
          default = 9944;
          description = "RPC server port";
        };
        
        logLevel = mkOption {
          type = types.str;
          default = "runtime=debug";
          description = "Log level configuration";
        };
        
        reservedNodes = mkOption {
          type = types.listOf types.str;
          default = [];
          example = ["/dns/dave.node.sc.iog.io/tcp/30333/p2p/12D3KooWH4LhgJDUbYbXsksQef4jTpDjA64ecUBjBVJprNzF64hE"];
          description = "List of reserved nodes";
        };
        
        bootNodes = mkOption {
          type = types.listOf types.str;
          default = [];
          example = ["/dns/eve.node.sc.iog.io/tcp/30333/p2p/12D3KooWN3YiYbk9nMZJ2VG7uk9iKfFWb1Kwrj7PoMdadfnAsRJm"];
          description = "List of boot nodes";
        };
        
        extraArgs = mkOption {
          type = types.listOf types.str;
          default = [];
          example = ["--rpc-methods=unsafe" "--rpc-max-connections" "1000"];
          description = "Additional command line arguments";
        };
        
        enableRpc = mkOption {
          type = types.bool;
          default = false;
          description = "Enable RPC server";
        };
        
        rpcCors = mkOption {
          type = types.str;
          default = "all";
          description = "RPC CORS setting";
        };
        
        blockBeneficiary = mkOption {
          type = types.str;
          example = "0a04cb23cff606facb13ddd43655840e9f6f32bd7b432809620d461596e188e9";
          description = "Sidechain block beneficiary address";
        };
        
        package = mkOption {
          type = types.package;
          default = pkgs.partner-chains.packages.x86_64-linux.partner-chains;
          description = "The partner-chains package to use";
        };
        
        # New options
        enableValidator = mkOption {
          type = types.bool;
          default = false;
          description = "Enable validator mode";
        };
        
        pruning = mkOption {
          type = types.enum [ "default" "archive" ];
          default = "default";
          description = "Pruning mode for the blockchain data";
        };
      };

      config = mkIf cfg.enable {
        systemd.services.partner-chains = {
          description = "Partner Chains Node";
          wantedBy = [ "multi-user.target" ];
          after = [ "network.target" ];
          
          # Combine the default environment with any overrides
          environment = cfg.environment // {
            SIDECHAIN_BLOCK_BENEFICIARY = cfg.blockBeneficiary;
          };
          
          preStart = ''
            # Ensure directories exist
            mkdir -p ${dirOf cfg.chainSpecPath}
            mkdir -p ${cfg.keystorePath}
            
            # Create a default chain spec if it doesn't exist
            if [ ! -f "${cfg.chainSpecPath}" ]; then
              echo "Chain spec file does not exist at ${cfg.chainSpecPath}"
              echo "Creating a minimal placeholder. Please ensure a proper chain spec is configured."
              echo '{"name": "partner-chains", "id": "partner-chains"}' > ${cfg.chainSpecPath}
            fi
            
            # Set proper permissions
            chown -R partner-chains:partner-chains ${dirOf cfg.chainSpecPath}
            chown -R partner-chains:partner-chains ${cfg.keystorePath}
            chmod 750 ${dirOf cfg.chainSpecPath}
            chmod 750 ${cfg.keystorePath}
            chmod 640 ${cfg.chainSpecPath}
            
            # Create data directories with proper permissions
            DATA_DIR=$(dirname ${cfg.keystorePath})
            if [ ! -d "$DATA_DIR" ]; then
              mkdir -p $DATA_DIR
              chown -R partner-chains:partner-chains $DATA_DIR
              chmod -R 750 $DATA_DIR
            fi
          '';
          
          serviceConfig = let
            # Node name flag (--alice, --bob, etc.)
            nodeNameFlag = if cfg.nodeName != null then "--${cfg.nodeName}" else "";
            
            # Reserved nodes flags
            reservedNodesFlags = if cfg.reservedNodes != [] 
              then ["--reserved-only"] ++ (flatten (map (node: ["--reserved-nodes" node]) cfg.reservedNodes))
              else [];
              
            # Boot nodes flags
            bootNodesFlags = flatten (map (node: ["--bootnodes" node]) cfg.bootNodes);
            
            # RPC flags
            rpcFlags = if cfg.enableRpc 
              then ["--rpc-external" "--rpc-cors=${cfg.rpcCors}" "--rpc-port" (toString cfg.rpcPort)]
              else [];
              
            # Pruning flags
            pruningFlags = if cfg.pruning == "archive" 
              then ["--state-pruning" "archive" "--blocks-pruning" "archive"]
              else [];
              
            # Combine all arguments
            allArgs = 
              [nodeNameFlag]
              ++ (optional cfg.enableValidator "--validator")
              ++ ["--node-key" cfg.nodeKey]
              ++ ["--chain" cfg.chainSpecPath]
              ++ reservedNodesFlags
              ++ bootNodesFlags
              ++ ["-llibp2p=debug"]
              ++ ["--listen-addr" cfg.listenAddr]
              ++ ["--keystore-path" cfg.keystorePath]
              ++ ["--log" cfg.logLevel]
              ++ ["--prometheus-port" (toString cfg.prometheusPort)]
              ++ ["--prometheus-external"]
              ++ rpcFlags
              ++ pruningFlags
              ++ cfg.extraArgs;
              
            # Filter out empty strings (from nodeNameFlag if it's empty)
            cleanArgs = filter (arg: arg != "") allArgs;
          in {
            ExecStart = "${cfg.package}/bin/partner-chains-node ${concatStringsSep " " cleanArgs}";
            
            # Important! Create a StateDirectory for persistent storage
            StateDirectory = "partner-chains";
            
            # This ensures systemd captures all stdout/stderr
            StandardOutput = "journal";
            StandardError = "journal";
            
            # Security hardening
            User = "partner-chains";
            Group = "partner-chains";
            Restart = "always";
            RestartSec = "10s";
            LimitNOFILE = 65535;
          };
        };

        # Create the user/group
        users.users.partner-chains = {
          isSystemUser = true;
          group = "partner-chains";
          home = "/var/lib/partner-chains";
          createHome = true;
        };
        users.groups.partner-chains = {};
        
        # Open firewall ports
        networking.firewall.allowedTCPPorts = [ 
          30333 # P2P port
          cfg.prometheusPort
        ] ++ (optionals cfg.enableRpc [ cfg.rpcPort ]);
      };
    };
} 