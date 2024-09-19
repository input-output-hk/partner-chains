{
  self,
  inputs,
  ...
}: {
  perSystem = {
    inputs',
    self',
    pkgs,
    system,
    ...
  }: let
    isDarwin = pkgs.lib.hasSuffix "darwin" system;
    fenixPkgs = inputs'.fenix.packages;
    rustToolchain = with fenixPkgs;
      fromToolchainFile {
        file = ../rust-toolchain.toml;
        # Probably should be a flake input instead
        sha256 = "+syqAd2kX8KVa8/U2gz3blIQTTsYYt3U63xBWaGOSc8=";
      };
    packages = with pkgs;
      [
        coreutils
        protobuf
        rustToolchain # The rust toolchain we constructed above
        nodejs
        clang
        nodePackages.npm
		gnumake
      ]
      ++ (
        if isDarwin
        then
          with pkgs.darwin.apple_sdk_11_0; [
            frameworks.SystemConfiguration
            frameworks.CoreFoundation
            darwin.Libsystem
          ]
        else []
      );
    env = [
      {
        name = "RUST_SRC_PATH";
        value = "${rustToolchain}/lib/rustlib/src/rust/library";
      }
      {
        name = "LIBCLANG_PATH";
        value = "${pkgs.libclang.lib}/lib";
      }
      {
        name = "LD_LIBRARY_PATH";
        value = "${rustToolchain}/lib";
      }
      {
        name = "BINDGEN_EXTRA_CLANG_ARGS";
        value = with pkgs;
          if isDarwin
          then "-isystem ${darwin.Libsystem}/include"
          else "-isystem ${libclang.lib}/lib/clang/${lib.getVersion libclang}/include";
      }
      {
        name = "PATH";
        prefix = "${pkgs.coreutils}/bin";
      }
    ];
    # Main Categories which can include pkgs, or devShell-like sets
    # for commands and helpers
    commands = self.lib.categorize [
      {
        help = "Earthly, an easy to use CI tool";
        name = "earthly";
        package = pkgs.earthly;
        category = "CI/CD";
        pkgs = [
          pkgs.earthly
          pkgs.awscli2
          pkgs.kubectl
          pkgs.kubernetes-helm
        ];
      }
      {
        category = "Rust";
        pkgs = [
          {
            name = "check";
            help = "Check rustc and clippy warnings";
            command = ''
              set -x
              cargo check --all-targets
              cargo clippy --all-targets
            '';
          }
          {
            help = "Automatically fix rustc and clippy warnings";
            name = "fix";
            command = ''
              set -x
              cargo fix --all-targets --allow-dirty --allow-staged
              cargo clippy --all-targets --fix --allow-dirty --allow-staged
            '';
          }
        ];
      }
      {
        category = "Partner Chains";
        pkgs = [
          {
            name = "cardano-cli";
            help = "CLI v9.1.0 that is used in partner-chains dependency stack";
            # This command has some eval because of IFD
            command = "${self'.packages.cardano-cli}/bin/cardano-cli $@";
          }
        ];
      }
    ];
    extraCommands =
      commands
      ++ self.lib.categorize [
        {
          category = "Partner Chains";
          pkgs = [
            {
              name = "partnerchains-stack";
              help = "Run a process-compose stack of all of the dependencies";
              command = ''
                ${self'.packages.partnerchains-stack}/bin/partnerchains-stack $@
              '';
            }
            {
              name = "pc-contracts-cli";
              help = "CLI to interact with Partner Chains Smart Contracts";
              command = ''
                ${self'.packages.pc-contracts-cli}/bin/pc-contracts-cli $@
              '';
            }
          ];
        }
      ];
  in {
    devshells.default = {
      inherit packages env commands;
      name = "Partner Chains Substrate Node Devshell";
    };
    devshells.process-compose = {
      inherit packages env;
      commands = extraCommands;
      name = "Partner Chains Substrate Node Devshell with whole stack";
    };
    devshells.smart-contracts = {
      inherit packages env;
      commands = commands ++ [
        {
          category = "Partner Chains";
          name = "pc-contracts-cli";
          help = "CLI to interact with Partner Chains Smart Contracts";
          command = ''
            ${self'.packages.pc-contracts-cli}/bin/pc-contracts-cli $@
          '';
        }
      ];
      name = "Partner Chains Substrate Node Devshell with Smart Contracts CLI";
    };
  };
}
